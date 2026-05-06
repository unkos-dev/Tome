//! `EnrichmentStatus` — closed value set for the Postgres `enrichment_status`
//! ENUM applied to `manifestations.enrichment_status`.
//!
//! Defensive type-safety (UNK-173, extends UNK-107): pre-migration,
//! reads decoded as `String` and matched against literals with a
//! `_ => {}` catch-all that silently dropped unknown variants.
//! `sqlx::Type` decode of an unknown DB variant now returns an error
//! rather than coercing into an unmatched string, and Rust-side
//! `match` arms are exhaustive at compile time.
//!
//! Wire formats:
//! - Postgres: `enrichment_status` ENUM (see migration
//!   `20260417120000_metadata_enrichment.up.sql`).
//! - JSON: lowercase string —
//!   `"pending"` | `"in_progress"` | `"complete"` | `"failed"` | `"skipped"`.

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, sqlx::Type,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "enrichment_status", rename_all = "snake_case")]
pub enum EnrichmentStatus {
    Pending,
    InProgress,
    Complete,
    Failed,
    Skipped,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip_uses_snake_case_string() {
        let status = EnrichmentStatus::InProgress;
        let json = serde_json::to_string(&status).expect("serialize");
        assert_eq!(json, "\"in_progress\"");
        let back: EnrichmentStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, EnrichmentStatus::InProgress);
    }

    #[test]
    fn json_rejects_unknown_variant() {
        let result: Result<EnrichmentStatus, _> = serde_json::from_str("\"resumed\"");
        assert!(result.is_err(), "expected resumed to be rejected");
    }
}
