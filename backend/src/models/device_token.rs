use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct DeviceToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    #[serde(skip)]
    pub token_hash: String,
    pub last_used_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
}

#[cfg(test)]
pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    token_hash: &str,
) -> Result<DeviceToken, sqlx::Error> {
    sqlx::query_as!(
        DeviceToken,
        "INSERT INTO device_tokens (user_id, name, token_hash) \
         VALUES ($1, $2, $3) \
         RETURNING id, user_id, name, token_hash, last_used_at, created_at, revoked_at",
        user_id,
        name,
        token_hash,
    )
    .fetch_one(pool)
    .await
}

/// List active (non-revoked) tokens for a user.
pub async fn list_for_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<DeviceToken>, sqlx::Error> {
    sqlx::query_as!(
        DeviceToken,
        "SELECT id, user_id, name, token_hash, last_used_at, created_at, revoked_at \
         FROM device_tokens \
         WHERE user_id = $1 AND revoked_at IS NULL \
         ORDER BY created_at DESC",
        user_id,
    )
    .fetch_all(pool)
    .await
}

/// Revoke a token. Scoped to `user_id` to prevent cross-user revocation.
pub async fn revoke(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE device_tokens SET revoked_at = now() \
         WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
        id,
        user_id,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

#[derive(Debug)]
pub enum CreateError {
    LimitExceeded,
    Db(sqlx::Error),
}

/// Atomically check the active token count and insert if under the limit.
/// Uses a transaction with SELECT FOR UPDATE to prevent TOCTOU races.
const MAX_TOKENS_PER_USER: i64 = 10;

pub async fn create_with_limit(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    token_hash: &str,
) -> Result<DeviceToken, CreateError> {
    let mut tx = pool.begin().await.map_err(CreateError::Db)?;

    // Lock the user's active token rows to serialize concurrent creates,
    // then count them in Rust. Postgres rejects `count(*) ... FOR UPDATE`
    // (aggregate + row lock) as a single statement, so we acquire the
    // locks and count the returned rows.
    let locked = sqlx::query!(
        "SELECT 1 AS dummy FROM device_tokens \
         WHERE user_id = $1 AND revoked_at IS NULL \
         FOR UPDATE",
        user_id,
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(CreateError::Db)?;

    if i64::try_from(locked.len()).unwrap_or(i64::MAX) >= MAX_TOKENS_PER_USER {
        return Err(CreateError::LimitExceeded);
    }

    let dt = sqlx::query_as!(
        DeviceToken,
        "INSERT INTO device_tokens (user_id, name, token_hash) \
         VALUES ($1, $2, $3) \
         RETURNING id, user_id, name, token_hash, last_used_at, created_at, revoked_at",
        user_id,
        name,
        token_hash,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(CreateError::Db)?;

    tx.commit().await.map_err(CreateError::Db)?;
    Ok(dt)
}

/// Update `last_used_at`, debounced SQL-side to at most one UPDATE per token
/// per 5 minutes. The WHERE predicate turns every call into a no-op when a
/// previous update landed within the window — single source of truth, atomic
/// under concurrent requests, no Rust-side policy to unit-test.
pub async fn update_last_used(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE device_tokens SET last_used_at = now() \
         WHERE id = $1 \
           AND (last_used_at IS NULL OR last_used_at < now() - interval '5 minutes')",
        id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "./migrations")]
    async fn create_list_revoke_lifecycle(pool: PgPool) {
        let oidc_subject = format!("token-test-{}", Uuid::new_v4());
        let user_id = sqlx::query_scalar!(
            "INSERT INTO users (oidc_subject, display_name) VALUES ($1, 'Token Test') RETURNING id",
            oidc_subject,
        )
        .fetch_one(&pool)
        .await
        .expect("create user");

        let token = create(&pool, user_id, "My Kindle", "fake-hash")
            .await
            .expect("create token");
        assert_eq!(token.name, "My Kindle");
        assert!(token.revoked_at.is_none());

        let tokens = list_for_user(&pool, user_id).await.expect("list");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].id, token.id);

        let revoked = revoke(&pool, token.id, user_id).await.expect("revoke");
        assert!(revoked);

        let tokens = list_for_user(&pool, user_id)
            .await
            .expect("list after revoke");
        assert!(tokens.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn list_for_user_excludes_revoked(pool: PgPool) {
        let oidc_subject = format!("revoke-filter-{}", Uuid::new_v4());
        let user_id = sqlx::query_scalar!(
            "INSERT INTO users (oidc_subject, display_name) VALUES ($1, 'Revoke Filter') RETURNING id",
            oidc_subject,
        )
        .fetch_one(&pool)
        .await
        .expect("create user");

        let active = create(&pool, user_id, "active", "hash-active")
            .await
            .expect("create active");
        let to_revoke = create(&pool, user_id, "to-revoke", "hash-revoked")
            .await
            .expect("create revoked");
        assert!(revoke(&pool, to_revoke.id, user_id).await.expect("revoke"),);

        let listed = list_for_user(&pool, user_id).await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, active.id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn update_last_used_debounced_within_window(pool: PgPool) {
        let oidc_subject = format!("debounce-{}", Uuid::new_v4());
        let user_id = sqlx::query_scalar!(
            "INSERT INTO users (oidc_subject, display_name) VALUES ($1, 'Debounce') RETURNING id",
            oidc_subject,
        )
        .fetch_one(&pool)
        .await
        .expect("create user");
        let token = create(&pool, user_id, "debounce", "hash-debounce")
            .await
            .expect("create token");

        update_last_used(&pool, token.id).await.expect("first");
        let first = sqlx::query_scalar!(
            "SELECT last_used_at FROM device_tokens WHERE id = $1",
            token.id,
        )
        .fetch_one(&pool)
        .await
        .expect("fetch first");
        let first = first.expect("first last_used_at not null");

        // Sleep 50ms then update again — the SQL predicate should veto the write
        // because last_used_at < now() - interval '5 minutes' is false.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        update_last_used(&pool, token.id).await.expect("second");
        let second = sqlx::query_scalar!(
            "SELECT last_used_at FROM device_tokens WHERE id = $1",
            token.id,
        )
        .fetch_one(&pool)
        .await
        .expect("fetch second")
        .expect("second last_used_at not null");
        assert_eq!(
            first, second,
            "second update within 5-minute window must be a no-op"
        );
    }
}
