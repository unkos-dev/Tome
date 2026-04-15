//! Write extracted metadata fields as individual `metadata_version` rows
//! (source = 'opf', status = 'draft').

use sqlx::PgPool;
use uuid::Uuid;

use super::extractor::ExtractedMetadata;

/// Write all non-None extracted fields as draft metadata_version rows.
pub async fn write_drafts(
    pool: &PgPool,
    manifestation_id: Uuid,
    metadata: &ExtractedMetadata,
) -> Result<(), sqlx::Error> {
    let confidence = metadata.confidence;

    if let Some(ref title) = metadata.title {
        insert_draft(
            pool,
            manifestation_id,
            "title",
            &json_string(title),
            confidence,
        )
        .await?;
    }
    if let Some(ref desc) = metadata.description {
        insert_draft(
            pool,
            manifestation_id,
            "description",
            &json_string(desc),
            confidence,
        )
        .await?;
    }
    if let Some(ref pub_name) = metadata.publisher {
        insert_draft(
            pool,
            manifestation_id,
            "publisher",
            &json_string(pub_name),
            confidence,
        )
        .await?;
    }
    if let Some(ref d) = metadata.pub_date {
        insert_draft(
            pool,
            manifestation_id,
            "pub_date",
            &json_string(&d.to_string()),
            confidence,
        )
        .await?;
    }
    if let Some(ref lang) = metadata.language {
        insert_draft(
            pool,
            manifestation_id,
            "language",
            &json_string(lang),
            confidence,
        )
        .await?;
    }
    if let Some(ref isbn) = metadata.isbn {
        if let Some(ref v) = isbn.isbn_10 {
            insert_draft(
                pool,
                manifestation_id,
                "isbn_10",
                &json_string(v),
                confidence,
            )
            .await?;
        }
        if let Some(ref v) = isbn.isbn_13 {
            insert_draft(
                pool,
                manifestation_id,
                "isbn_13",
                &json_string(v),
                confidence,
            )
            .await?;
        }
    }
    if !metadata.creators.is_empty() {
        let val = serde_json::to_value(&metadata.creators).unwrap_or_default();
        insert_draft(pool, manifestation_id, "creators", &val, confidence).await?;
    }
    if !metadata.subjects.is_empty() {
        let val = serde_json::to_value(&metadata.subjects).unwrap_or_default();
        insert_draft(pool, manifestation_id, "subjects", &val, confidence).await?;
    }
    if let Some(ref series) = metadata.series {
        let val = serde_json::to_value(series).unwrap_or_default();
        insert_draft(pool, manifestation_id, "series", &val, confidence).await?;
    }
    if let Some(ref inv) = metadata.inversion {
        let val = serde_json::json!({
            "probable_author": inv.probable_author,
            "probable_title": inv.probable_title,
        });
        insert_draft(
            pool,
            manifestation_id,
            "inversion_detected",
            &val,
            confidence * 0.5,
        )
        .await?;
    }

    Ok(())
}

async fn insert_draft(
    pool: &PgPool,
    manifestation_id: Uuid,
    field_name: &str,
    new_value: &serde_json::Value,
    confidence: f32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO metadata_versions (manifestation_id, source, field_name, new_value, confidence_score) \
         VALUES ($1, 'opf'::metadata_source, $2, $3, $4)",
    )
    .bind(manifestation_id)
    .bind(field_name)
    .bind(new_value)
    .bind(confidence)
    .execute(pool)
    .await?;
    Ok(())
}

fn json_string(s: &str) -> serde_json::Value {
    serde_json::Value::String(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::metadata::extractor::{ExtractedCreator, ExtractedMetadata, SeriesInfo};
    use crate::services::metadata::isbn::IsbnResult;

    fn db_url() -> String {
        std::env::var("DATABASE_URL_INGESTION").unwrap_or_else(|_| {
            "postgres://tome_ingestion:tome_ingestion@localhost:5433/tome_dev".into()
        })
    }

    /// Create a work + manifestation to get a valid manifestation_id for testing.
    async fn setup_manifestation(pool: &PgPool) -> (Uuid, Uuid) {
        let work_id: Uuid = sqlx::query_scalar(
            "INSERT INTO works (title, sort_title) VALUES ('draft_test', 'draft_test') RETURNING id",
        )
        .fetch_one(pool)
        .await
        .unwrap();

        let manifestation_id: Uuid = sqlx::query_scalar(
            "INSERT INTO manifestations \
             (work_id, format, file_path, file_hash, file_size_bytes, \
              ingestion_status, validation_status) \
             VALUES ($1, 'epub'::manifestation_format, $2, 'hash', 100, \
                     'complete'::ingestion_status, 'valid'::validation_status) \
             RETURNING id",
        )
        .bind(work_id)
        .bind(format!("/tmp/draft-test-{work_id}.epub"))
        .fetch_one(pool)
        .await
        .unwrap();

        (work_id, manifestation_id)
    }

    async fn cleanup(pool: &PgPool, work_id: Uuid, manifestation_id: Uuid) {
        let _ = sqlx::query("DELETE FROM metadata_versions WHERE manifestation_id = $1")
            .bind(manifestation_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM manifestations WHERE id = $1")
            .bind(manifestation_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM works WHERE id = $1")
            .bind(work_id)
            .execute(pool)
            .await;
    }

    #[tokio::test]
    #[ignore] // requires PostgreSQL with migrations applied
    async fn write_drafts_creates_metadata_version_rows() {
        let pool = PgPool::connect(&db_url()).await.unwrap();
        let (work_id, manifestation_id) = setup_manifestation(&pool).await;

        let metadata = ExtractedMetadata {
            title: Some("Draft Test Title".into()),
            sort_title: Some("draft test title".into()),
            description: Some("A description".into()),
            language: Some("en".into()),
            creators: vec![ExtractedCreator {
                name: "Test Writer".into(),
                sort_name: "Writer, Test".into(),
                role: "author".into(),
            }],
            publisher: Some("Test Publisher".into()),
            pub_date: None,
            isbn: Some(IsbnResult {
                isbn_10: None,
                isbn_13: Some("9780306406157".into()),
                valid: true,
            }),
            subjects: vec!["Fiction".into()],
            series: Some(SeriesInfo {
                name: "Test Series".into(),
                position: Some(1.0),
            }),
            inversion: None,
            confidence: 0.7,
        };

        write_drafts(&pool, manifestation_id, &metadata)
            .await
            .unwrap();

        // Count rows — should have: title, description, language, publisher, isbn_13, creators, subjects, series = 8
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM metadata_versions WHERE manifestation_id = $1",
        )
        .bind(manifestation_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(count >= 8, "expected at least 8 draft rows, got {count}");

        // Verify source is 'opf' and status is 'draft'
        let non_draft: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM metadata_versions \
             WHERE manifestation_id = $1 AND (source::text != 'opf' OR status::text != 'draft')",
        )
        .bind(manifestation_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(non_draft, 0, "all rows should be source=opf, status=draft");

        // Verify confidence
        let confidence: f32 = sqlx::query_scalar(
            "SELECT confidence_score FROM metadata_versions \
             WHERE manifestation_id = $1 AND field_name = 'title'",
        )
        .bind(manifestation_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!((confidence - 0.7).abs() < 0.01);

        cleanup(&pool, work_id, manifestation_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn write_drafts_skips_none_fields() {
        let pool = PgPool::connect(&db_url()).await.unwrap();
        let (work_id, manifestation_id) = setup_manifestation(&pool).await;

        // Minimal metadata — only title
        let metadata = ExtractedMetadata {
            title: Some("Minimal".into()),
            sort_title: Some("minimal".into()),
            description: None,
            language: None,
            creators: vec![],
            publisher: None,
            pub_date: None,
            isbn: None,
            subjects: vec![],
            series: None,
            inversion: None,
            confidence: 0.3,
        };

        write_drafts(&pool, manifestation_id, &metadata)
            .await
            .unwrap();

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM metadata_versions WHERE manifestation_id = $1",
        )
        .bind(manifestation_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "only title should be written");

        cleanup(&pool, work_id, manifestation_id).await;
    }
}
