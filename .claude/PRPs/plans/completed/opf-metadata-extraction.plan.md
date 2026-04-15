# Plan: OPF Metadata Extraction and ISBN Validation

## Summary

Extract Dublin Core metadata from EPUB OPF files during ingestion, validate ISBNs, sanitise text fields, detect title-author inversion, and store all extracted metadata as draft `metadata_version` rows. Replace the current blind `INSERT INTO works` with intelligent work-matching (ISBN then pg_trgm fuzzy) and proper author/series record creation.

## User Story

As a library owner,
I want books ingested with correct metadata extracted from their OPF files,
So that my library is browsable by real title, author, and series — not just filenames.

## Problem → Solution

**Current:** Ingestion creates a Work with `title = filename heuristic` and no author, ISBN, publisher, or series data. The manifestation row has NULL isbn_10/13, publisher, pub_date.

**Desired:** Ingestion extracts all Dublin Core metadata from the OPF, validates ISBNs, creates proper Author records, links to Series, deduplicates against existing Works, and stores every extracted field as an auditable draft `metadata_version`. Path templates render with real metadata.

## Metadata

- **Complexity**: Large
- **Source PRD**: `plans/BLUEPRINT.md`
- **PRD Phase**: Step 6 — OPF Metadata Extraction and ISBN Validation
- **Estimated Files**: 12-15 (8 new, 4-7 modified)

---

## UX Design

N/A — internal change. No user-facing UI in this step.

---

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---|---|
| P0 | `backend/src/services/epub/opf_layer.rs` | all | OPF XML parsing loop — must extend to extract DC fields |
| P0 | `backend/src/services/epub/mod.rs` | all | ValidationReport struct — must carry OpfData |
| P0 | `backend/src/services/ingestion/orchestrator.rs` | 229-439 | process_file pipeline — CTE rewrite target |
| P1 | `backend/migrations/20260412150002_core_tables.up.sql` | all | works, authors, work_authors, manifestations schema |
| P1 | `backend/migrations/20260412150003_series_and_metadata.up.sql` | all | metadata_versions, series, series_works schema |
| P1 | `backend/migrations/20260412150007_search_rls_and_reserved.up.sql` | 1-30 | GIN/GIST indexes for trgm + ISBN |
| P1 | `backend/src/services/ingestion/path_template.rs` | all | Template rendering + heuristic_vars_from_filename |
| P2 | `backend/src/models/ingestion_job.rs` | all | Model pattern: sqlx::FromRow, async fns, pool param |
| P2 | `backend/src/error.rs` | all | AppError enum pattern |
| P2 | `backend/src/config.rs` | all | Config struct, env loading |

## External Documentation

| Topic | Source | Key Takeaway |
|---|---|---|
| Dublin Core in OPF | EPUB 3.3 spec, OPF package document | DC elements live under `<metadata>` in `dc:` namespace. EPUB 2 uses `<dc:creator opf:role="aut">`, EPUB 3 uses `<meta refines="#id" property="role">` |
| ISBN check digits | ISO 2108 | ISBN-10: mod-11 with X=10. ISBN-13: alternating 1/3 weights mod-10 |
| Calibre series meta | Calibre docs | `<meta name="calibre:series" content="..."/>` + `<meta name="calibre:series_index" content="..."/>` |
| EPUB 3 collections | EPUB 3.3 spec | `<meta property="belongs-to-collection" id="c01">Series Name</meta>` + `<meta refines="#c01" property="group-position">1</meta>` |
| pg_trgm similarity | PostgreSQL docs | `similarity(a, b)` returns 0.0-1.0. Default threshold 0.3. GiST index on `gist_trgm_ops` |

---

## Patterns to Mirror

### NAMING_CONVENTION
```rust
// SOURCE: backend/src/services/epub/opf_layer.rs:10-18
// Struct names: PascalCase. Fields: snake_case. Modules: snake_case.
pub struct OpfData {
    pub manifest: HashMap<String, String>,
    pub spine_idrefs: Vec<String>,
    pub opf_path: String,
    pub accessibility_metadata: Option<serde_json::Value>,
}
```

### ERROR_HANDLING
```rust
// SOURCE: backend/src/error.rs:4-16
// Use thiserror for enum, anyhow for wrapping. Internal errors log but don't leak.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}
```

### LOGGING_PATTERN
```rust
// SOURCE: backend/src/services/ingestion/orchestrator.rs:353-358
// Use tracing macros with structured fields
tracing::info!(
    path = %lib_file.display(),
    outcome = ?report.outcome,
    issues = report.issues.len(),
    "epub validation complete"
);
```

### MODEL_PATTERN
```rust
// SOURCE: backend/src/models/ingestion_job.rs:6-16
// sqlx::FromRow for compile-time-checked queries. Enums stored as String (cast from SQL enum).
#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct IngestionJob {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub source_path: String,
    pub status: String,                  // SQL enum cast to ::text
    pub error_message: Option<String>,
    pub started_at: Option<OffsetDateTime>,
    pub completed_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}
```

### DB_QUERY_PATTERN
```rust
// SOURCE: backend/src/models/ingestion_job.rs:18-33
// Inline SQL strings with sqlx::query_as, RETURNING clause, enum casts
pub async fn create(pool: &PgPool, batch_id: Uuid, source_path: &str) -> Result<IngestionJob, sqlx::Error> {
    sqlx::query_as::<_, IngestionJob>(
        "INSERT INTO ingestion_jobs (batch_id, source_path) \
         VALUES ($1, $2) \
         RETURNING id, batch_id, source_path, status::text, error_message, \
                   started_at, completed_at, created_at",
    )
    .bind(batch_id)
    .bind(source_path)
    .fetch_one(pool)
    .await
}
```

### ORCHESTRATOR_SPAWN_BLOCKING
```rust
// SOURCE: backend/src/services/ingestion/orchestrator.rs:346-349
// CPU-bound work in spawn_blocking, async result unwrapped outside
let validation = {
    let lib_file = lib_file.clone();
    tokio::task::spawn_blocking(move || epub::validate_and_repair(&lib_file)).await
};
```

### TEST_STRUCTURE
```rust
// SOURCE: backend/src/services/ingestion/orchestrator.rs:453-461
// Inline #[cfg(test)] mod. Integration tests: #[ignore], PgPool from env, tempdir, cleanup.
#[cfg(test)]
mod tests {
    use super::*;

    fn db_url() -> String {
        std::env::var("DATABASE_URL_INGESTION").unwrap_or_else(|_| {
            "postgres://tome_ingestion:tome_ingestion@localhost:5433/tome_dev".into()
        })
    }
    // ... tests with cleanup_test_data() at end
}
```

### OPF_XML_EVENT_LOOP
```rust
// SOURCE: backend/src/services/epub/opf_layer.rs:38-128
// quick-xml event-driven loop with guarded match arms. Meta before general.
loop {
    match reader.read_event().ok()? {
        Event::Start(e) if e.name().as_ref() == b"meta" => { /* ... */ }
        Event::Empty(e) if e.name().as_ref() == b"meta" => { /* ... */ }
        Event::Empty(e) | Event::Start(e) => match e.name().as_ref() {
            b"item" => { /* ... */ }
            b"itemref" => { /* ... */ }
            _ => {}
        },
        Event::Eof => break,
        _ => {}
    }
}
```

---

## Files to Change

| File | Action | Justification |
|---|---|---|
| `backend/src/services/epub/opf_layer.rs` | UPDATE | Extend XML event loop to extract DC metadata into OpfData |
| `backend/src/services/epub/mod.rs` | UPDATE | Add `opf_data: Option<OpfData>` to ValidationReport; make OpfData public |
| `backend/src/services/metadata/mod.rs` | CREATE | Module root: re-exports for extractor, isbn, sanitiser, inversion, draft |
| `backend/src/services/metadata/extractor.rs` | CREATE | Extract structured metadata from OpfData into ExtractedMetadata |
| `backend/src/services/metadata/isbn.rs` | CREATE | ISBN-10/ISBN-13 checksum validation and conversion |
| `backend/src/services/metadata/sanitiser.rs` | CREATE | Strip HTML, normalise whitespace, decode entities |
| `backend/src/services/metadata/inversion.rs` | CREATE | Heuristic title-author inversion detection |
| `backend/src/services/metadata/draft.rs` | CREATE | Write metadata_version rows (source=opf, status=draft) |
| `backend/src/services/mod.rs` | UPDATE | Add `pub mod metadata;` |
| `backend/src/models/work.rs` | CREATE | Work matching: ISBN lookup + pg_trgm fuzzy match |
| `backend/src/models/mod.rs` | UPDATE | Add `pub mod work;` |
| `backend/src/services/ingestion/orchestrator.rs` | UPDATE | Replace CTE with metadata-aware work matching + record creation |
| `backend/src/services/ingestion/path_template.rs` | UPDATE | Re-render path with extracted metadata after DB insert; atomic rename |

## NOT Building

- Author surname list / bundled name database (deferrable; comma heuristic is sufficient for MVP)
- Metadata enrichment from external APIs (Step 7)
- Metadata writeback to EPUB files (Step 8)
- UI for accepting/rejecting metadata drafts (Step 10)
- NER-based inversion detection (Phase 2+)
- Work matching by fuzzy ISBN (e.g., ISBN-10 ↔ ISBN-13 cross-lookup) — only exact match for now
- Sort title generation beyond lowercasing (article stripping "The", "A" is nice-to-have, not blocking)

---

## Step-by-Step Tasks

### Task 1: Extend OpfData with Dublin Core fields

- **ACTION**: Add DC metadata fields to `OpfData` struct and extract them in the XML event loop
- **IMPLEMENT**:
  Add to `OpfData`:
  ```rust
  pub title: Option<String>,
  pub creators: Vec<Creator>,     // name + optional role (aut/edt/trl/nrt)
  pub description: Option<String>,
  pub publisher: Option<String>,
  pub date: Option<String>,       // raw string, parsed later
  pub language: Option<String>,
  pub identifiers: Vec<String>,   // all dc:identifier values (ISBNs, URNs, etc.)
  pub subjects: Vec<String>,      // dc:subject values
  pub series_meta: Option<SeriesMeta>, // calibre or EPUB3 collection
  ```
  New structs (in opf_layer.rs):
  ```rust
  #[derive(Debug, Clone)]
  pub struct Creator {
      pub name: String,
      pub role: Option<String>,  // opf:role attribute or refines
  }

  #[derive(Debug, Clone)]
  pub struct SeriesMeta {
      pub name: String,
      pub position: Option<f64>,
  }
  ```
  In the XML event loop, add match arms for DC elements (`dc:title`, `dc:creator`, `dc:description`, `dc:publisher`, `dc:date`, `dc:language`, `dc:identifier`, `dc:subject`). These appear as `Event::Start` with text content read via `reader.read_text()`. Also match calibre meta `name="calibre:series"` / `name="calibre:series_index"` in the existing Empty meta arm. For EPUB 3 `belongs-to-collection`, capture in the Start meta arm.
  Handle namespace: element local name after `:` — match both `b"title"` and `b"dc:title"` since `quick-xml` may or may not include the prefix depending on namespace handling.
- **MIRROR**: OPF_XML_EVENT_LOOP — guarded match arms, `e.into_owned()` before `read_text`
- **IMPORTS**: No new imports needed; all types already available
- **GOTCHA**: `reader.read_text()` consumes up to the matching end tag — must call it for Start events of DC elements to advance the reader correctly. For `Event::Empty` DC elements (rare but valid), there's no text content.
- **GOTCHA**: The `opf:role` attribute on `dc:creator` is EPUB 2 only. EPUB 3 uses `<meta refines="#creator01" property="role">aut</meta>`. For MVP, extract `opf:role` attribute and match by refines ID in a second pass if needed. Document that EPUB 3 role refinement is best-effort.
- **VALIDATE**: Extend existing `make_handle` tests: OPF with full DC metadata → all fields populated. OPF with empty metadata → all fields None/empty Vec. OPF with calibre series meta → SeriesMeta extracted.

### Task 2: Carry OpfData through ValidationReport

- **ACTION**: Add `opf_data: Option<OpfData>` to `ValidationReport` and populate it
- **IMPLEMENT**:
  In `epub/mod.rs`, add field:
  ```rust
  pub struct ValidationReport {
      pub issues: Vec<Issue>,
      pub outcome: ValidationOutcome,
      pub accessibility_metadata: Option<serde_json::Value>,
      pub opf_data: Option<opf_layer::OpfData>,  // NEW
  }
  ```
  Make `OpfData`, `Creator`, `SeriesMeta` public (they're already in a `pub mod`).
  In `validate_and_repair`, pass `opf_data` through to the return value. Remove the separate `accessibility_metadata` field from `ValidationReport` since it's already inside `OpfData` — or keep both for backward compat (the orchestrator currently reads `report.accessibility_metadata`). **Decision: keep both** to avoid touching the orchestrator's existing accessibility_metadata plumbing in this task.
  Update all `ValidationReport` construction sites (5 return paths in `validate_and_repair`) to include `opf_data: opf_data.clone()` or `opf_data: None` for early-exit quarantine paths.
- **MIRROR**: NAMING_CONVENTION — public struct fields
- **IMPORTS**: None new
- **GOTCHA**: `OpfData` contains `HashMap` which doesn't impl `Copy`. Use `.clone()` since it's only called once per file. The `opf_data` variable is `Option<OpfData>` — clone the whole Option.
- **VALIDATE**: `cargo build` — no compilation errors. Existing tests still pass.

### Task 3: Create ISBN validation module

- **ACTION**: Create `src/services/metadata/isbn.rs` with pure checksum validation
- **IMPLEMENT**:
  ```rust
  /// Strip hyphens and spaces, uppercase for ISBN-10 X digit.
  pub fn normalise(raw: &str) -> String

  /// Validate ISBN-10 checksum (mod-11, X=10 for check digit).
  pub fn validate_isbn10(isbn: &str) -> bool

  /// Validate ISBN-13 checksum (alternating 1/3 weights, mod-10).
  pub fn validate_isbn13(isbn: &str) -> bool

  /// Convert valid ISBN-10 to ISBN-13. Returns None if input invalid.
  pub fn isbn10_to_isbn13(isbn10: &str) -> Option<String>

  /// Parse a raw identifier string: strip prefixes ("urn:isbn:", "isbn:"),
  /// normalise, detect length (10 or 13), validate checksum.
  /// Returns (Option<isbn_10>, Option<isbn_13>, is_valid).
  pub fn parse_isbn(raw: &str) -> IsbnResult

  pub struct IsbnResult {
      pub isbn_10: Option<String>,
      pub isbn_13: Option<String>,
      pub valid: bool,
  }
  ```
  ISBN-10 algorithm: sum of `digit[i] * (10 - i)` for i=0..9, mod 11 == 0. Check digit may be 'X' (value 10).
  ISBN-13 algorithm: sum of `digit[i] * weight[i]` where weight alternates 1, 3, mod 10 == 0.
- **MIRROR**: No codebase pattern needed — pure functions with unit tests
- **IMPORTS**: None — pure std
- **GOTCHA**: Some OPF identifiers have prefixes like `urn:isbn:978...` or `isbn:978...` — strip these. Some have random UUIDs or URNs that aren't ISBNs at all — parse_isbn must return `valid: false` gracefully.
- **VALIDATE**: Unit tests covering: valid ISBN-10 ("0-306-40615-2"), valid ISBN-13 ("978-0-306-40615-7"), invalid checksum, ISBN-10→13 conversion, non-ISBN identifiers, empty string, hyphens/spaces.

### Task 4: Create metadata sanitiser

- **ACTION**: Create `src/services/metadata/sanitiser.rs` for text cleanup
- **IMPLEMENT**:
  ```rust
  /// Strip HTML tags from a string (description fields often contain HTML).
  pub fn strip_html(input: &str) -> String

  /// Normalise whitespace: collapse runs of \s+ to single space, trim.
  pub fn normalise_whitespace(input: &str) -> String

  /// Decode common HTML entities (&amp; &lt; &gt; &quot; &apos; &#NNN; &#xHH;).
  pub fn decode_entities(input: &str) -> String

  /// Full sanitisation pipeline: decode entities → strip HTML → normalise whitespace.
  pub fn sanitise(input: &str) -> String
  ```
  For `strip_html`: simple state machine — track whether inside `<...>` and skip those characters. No need for a full HTML parser; OPF descriptions contain basic HTML (p, br, em, strong). Don't use quick-xml for this — it would choke on HTML fragments that aren't valid XML.
  For `decode_entities`: handle named entities (&amp;, &lt;, &gt;, &quot;, &apos;) and numeric (&#NNN;, &#xHH;) via a small match + char::from_u32.
- **MIRROR**: Pure functions, no codebase pattern dependency
- **IMPORTS**: None — pure std
- **GOTCHA**: Some descriptions contain `<br/>` or `<br>` — both must be stripped. Some contain CDATA sections — strip the `<![CDATA[` and `]]>` wrappers too.
- **VALIDATE**: Unit tests: HTML paragraph → plain text, nested tags, entities, Word markup artifacts (`<o:p>`, `<w:...>`), empty input, already-clean text unchanged.

### Task 5: Create title-author inversion detector

- **ACTION**: Create `src/services/metadata/inversion.rs` with heuristic detection
- **IMPLEMENT**:
  ```rust
  /// Check if a title looks like it's actually an author name ("Lastname, Firstname").
  /// Returns Some((probable_author, probable_title)) if inversion detected.
  pub fn detect_inversion(title: &str, authors: &[String]) -> Option<InversionResult>

  pub struct InversionResult {
      pub probable_author: String,
      pub probable_title: String,
  }
  ```
  Heuristics:
  1. Title matches `Lastname, Firstname` pattern (comma with exactly two parts, first part is a single capitalised word)
  2. Title matches one of the declared authors (case-insensitive)
  3. An author field looks like a book title (contains articles, conjunctions, >4 words)

  This is advisory-only — the result is stored as a metadata_version draft with a lower confidence score, not auto-applied.
- **MIRROR**: Pure functions
- **IMPORTS**: None
- **GOTCHA**: Don't be too aggressive — "Smith, John" is a likely inversion but "Murder, She Wrote" is a valid title. Require the comma pattern AND check that the pre-comma part looks like a surname (single word, capitalised, <20 chars).
- **VALIDATE**: Unit tests: "Smith, John" with author "A Book Title" → detected. "The Great Gatsby" with author "F. Scott Fitzgerald" → not detected. "Murder, She Wrote" → not detected (post-comma part >1 word).

### Task 6: Create metadata extractor (orchestration layer)

- **ACTION**: Create `src/services/metadata/extractor.rs` that transforms OpfData into structured metadata
- **IMPLEMENT**:
  ```rust
  /// Structured metadata extracted from an OPF file, ready for DB storage.
  #[derive(Debug, Clone)]
  pub struct ExtractedMetadata {
      pub title: Option<String>,
      pub sort_title: Option<String>,
      pub description: Option<String>,
      pub language: Option<String>,
      pub creators: Vec<ExtractedCreator>,
      pub publisher: Option<String>,
      pub pub_date: Option<time::Date>,        // parsed from OPF date string
      pub isbn: Option<isbn::IsbnResult>,
      pub subjects: Vec<String>,
      pub series: Option<SeriesInfo>,
      pub inversion: Option<inversion::InversionResult>,
      pub confidence: f32,                     // 0.0-1.0 based on field completeness
  }

  #[derive(Debug, Clone)]
  pub struct ExtractedCreator {
      pub name: String,
      pub sort_name: String,  // "Firstname Lastname" → "Lastname, Firstname"
      pub role: String,       // default "author" if not specified
  }

  #[derive(Debug, Clone)]
  pub struct SeriesInfo {
      pub name: String,
      pub position: Option<f64>,
  }

  /// Extract and sanitise metadata from parsed OPF data.
  pub fn extract(opf: &opf_layer::OpfData) -> ExtractedMetadata
  ```
  Pipeline:
  1. Sanitise title, description, publisher via `sanitiser::sanitise()`
  2. Parse date string → `time::Date` (try multiple formats: "YYYY-MM-DD", "YYYY-MM", "YYYY", ISO 8601)
  3. Parse identifiers → `isbn::parse_isbn()` for each, keep the first valid one
  4. Map creators: sanitise names, generate sort_name ("Firstname Lastname" → "Lastname, Firstname"), map opf:role to author_role enum string
  5. Extract series from OpfData.series_meta
  6. Run inversion detection
  7. Compute confidence: base 0.3, +0.1 per present field (title, author, ISBN, publisher, date, description), cap at 1.0
- **MIRROR**: NAMING_CONVENTION for struct definitions
- **IMPORTS**: `use super::{isbn, sanitiser, inversion};` and `use crate::services::epub::opf_layer;`
- **GOTCHA**: Date parsing must not panic on garbage input — return None. The `time` crate (NOT chrono) is used in this project. Use `time::Date::parse()` with format descriptions.
- **GOTCHA**: Sort name generation: split on last space → "Tolkien, J. R. R." for "J. R. R. Tolkien". Handle single-word names (just use the name as-is for sort_name).
- **VALIDATE**: Unit tests with mock OpfData structs covering: full metadata, minimal metadata, garbage dates, ISBNs with prefixes, multi-author with roles.

### Task 7: Create draft metadata writer

- **ACTION**: Create `src/services/metadata/draft.rs` — async DB writes for metadata_version rows
- **IMPLEMENT**:
  ```rust
  /// Write all extracted fields as individual metadata_version rows.
  /// Each field gets its own row: field_name = "title", "description", etc.
  /// old_value = NULL (first extraction), new_value = JSONB of the value.
  pub async fn write_drafts(
      pool: &PgPool,
      manifestation_id: Uuid,
      metadata: &ExtractedMetadata,
  ) -> Result<(), sqlx::Error>
  ```
  For each non-None field in ExtractedMetadata, insert a row:
  ```sql
  INSERT INTO metadata_versions (manifestation_id, source, field_name, new_value, confidence_score)
  VALUES ($1, 'opf'::metadata_source, $2, $3, $4)
  ```
  Field mappings:
  - "title" → `serde_json::Value::String(title)`
  - "description" → `serde_json::Value::String(description)`
  - "publisher" → `serde_json::Value::String(publisher)`
  - "pub_date" → `serde_json::Value::String(date.to_string())`
  - "language" → `serde_json::Value::String(language)`
  - "isbn_10" → `serde_json::Value::String(isbn_10)`
  - "isbn_13" → `serde_json::Value::String(isbn_13)`
  - "creators" → `serde_json::to_value(&creators)` (array of objects)
  - "subjects" → `serde_json::to_value(&subjects)` (array of strings)
  - "series" → `serde_json::to_value(&series)` (object with name + position)
  - "inversion_detected" → if inversion present, store the detection result
- **MIRROR**: DB_QUERY_PATTERN — sqlx::query with bind params, enum casts
- **IMPORTS**: `use sqlx::PgPool; use uuid::Uuid; use serde_json;`
- **GOTCHA**: Batch inserts — use a single query with `UNNEST` arrays or a loop of individual inserts. Individual inserts are simpler and there are at most ~12 fields per manifestation. Use a loop.
- **VALIDATE**: Integration test (requires DB): insert drafts for a manifestation, query metadata_versions, verify rows exist with correct source='opf' and status='draft'.

### Task 8: Create work matching model

- **ACTION**: Create `src/models/work.rs` — find or create Work + Author records
- **IMPLEMENT**:
  ```rust
  /// Attempt to find an existing Work by ISBN, then by title+author similarity.
  /// If no match, create a new Work, Author(s), and work_authors joins.
  /// Returns the work_id.
  pub async fn find_or_create(
      pool: &PgPool,
      metadata: &ExtractedMetadata,
  ) -> Result<Uuid, sqlx::Error>
  ```
  Matching cascade (in a single transaction):
  1. **ISBN match**: If metadata has valid isbn_13, query:
     ```sql
     SELECT w.id FROM works w
     JOIN manifestations m ON m.work_id = w.id
     WHERE m.isbn_13 = $1
     LIMIT 1
     ```
  2. **Title+Author fuzzy match** (if no ISBN match): If metadata has title and at least one author:
     ```sql
     SELECT w.id, similarity(w.title, $1) AS title_sim
     FROM works w
     JOIN work_authors wa ON wa.work_id = w.id
     JOIN authors a ON a.id = wa.author_id
     WHERE similarity(w.title, $1) > 0.6
       AND similarity(a.name, $2) > 0.6
     ORDER BY title_sim DESC
     LIMIT 1
     ```
     Threshold 0.6 is conservative — avoids false positives while catching minor variations.
  3. **Create new**: If no match found, within the same transaction:
     ```sql
     INSERT INTO works (title, sort_title, description, language)
     VALUES ($1, $2, $3, $4) RETURNING id
     ```
     For each creator:
     ```sql
     INSERT INTO authors (name, sort_name)
     VALUES ($1, $2)
     ON CONFLICT DO NOTHING  -- authors table has no unique constraint yet, so this is a plain insert
     RETURNING id
     ```
     (Note: authors table has no unique constraint on name. For MVP, always insert. Dedup is a Step 7+ concern.)
     ```sql
     INSERT INTO work_authors (work_id, author_id, role, position)
     VALUES ($1, $2, $3::author_role, $4)
     ```
  4. **Series linking**: If metadata has series info:
     ```sql
     INSERT INTO series (name, sort_name)
     VALUES ($1, $2)
     ON CONFLICT DO NOTHING  -- no unique constraint, plain insert for MVP
     RETURNING id
     ```
     Then:
     ```sql
     INSERT INTO series_works (series_id, work_id, position)
     VALUES ($1, $2, $3)
     ON CONFLICT DO NOTHING
     ```
- **MIRROR**: MODEL_PATTERN, DB_QUERY_PATTERN
- **IMPORTS**: `use sqlx::PgPool; use uuid::Uuid; use crate::services::metadata::extractor::ExtractedMetadata;`
- **GOTCHA**: The whole find-or-create must be in a transaction. Use `pool.begin()` → `tx.commit()`. Two concurrent ingestions of the same book without this will create duplicate Works.
- **GOTCHA**: `similarity()` requires the `pg_trgm` extension — already enabled in migration 1. The GIST indexes on `works.title` and `authors.name` exist but only accelerate the `%` operator, not arbitrary `similarity() > threshold` WHERE clauses. At MVP data volumes this is fine; if perf becomes an issue, switch to `WHERE title % $1` with `SET pg_trgm.similarity_threshold`.
- **GOTCHA**: `author_role` enum values are: author, editor, translator, narrator. Map from OPF roles: "aut"→author, "edt"→editor, "trl"→translator, "nrt"→narrator, anything else→author.
- **VALIDATE**: Integration test: create Work via find_or_create, call again with same ISBN → returns same work_id. Call with different ISBN but similar title → creates new. Call with very similar title+author → returns match.

### Task 9: Create manifestation with extracted metadata and metadata-based path

- **ACTION**: Update the orchestrator to extract metadata, compute the final path, rename file, and create a properly populated manifestation in one flow
- **IMPLEMENT**:
  Replace the CTE in `process_file` (orchestrator.rs lines 397-418) with:
  1. If EPUB and `report.opf_data` is Some: call `metadata::extractor::extract(&opf_data)` to get ExtractedMetadata
  2. **Compute metadata-based path before insert**: Build vars from extracted metadata (`Author` = first creator's sort_name, `Title` = extracted title, `ext` = file extension). Re-render path template. If the new path differs from the heuristic path:
     a. Resolve collision on new path
     b. Create parent directories (`std::fs::create_dir_all`)
     c. Atomic rename (`std::fs::rename`) — same filesystem guaranteed since both are under library_path
     d. Clean up empty parent directories of the old path
     e. If rename fails, log warning and keep the heuristic path — the INSERT uses whichever path the file is actually at
  3. Call `work::find_or_create(pool, &metadata)` to get work_id
  4. Insert manifestation with full metadata and the **final** file_path (post-rename):
     ```sql
     INSERT INTO manifestations
         (work_id, isbn_10, isbn_13, publisher, pub_date, format,
          file_path, file_hash, file_size_bytes, ingestion_status,
          validation_status, accessibility_metadata)
     VALUES ($1, $2, $3, $4, $5, $6::manifestation_format, $7, $8, $9,
             'complete'::ingestion_status, $10::validation_status, $11)
     RETURNING id
     ```
  5. Call `draft::write_drafts(pool, manifestation_id, &metadata)` to create metadata_version rows
  6. For non-EPUB formats or when OpfData is None: fall back to current behavior (filename heuristic → Work with title only, no rename)

  This approach inserts the correct file_path on the first write — no UPDATE needed, no window where the DB points at a stale path.
- **MIRROR**: ORCHESTRATOR_SPAWN_BLOCKING for the extraction (it's sync code), DB_QUERY_PATTERN for queries
- **IMPORTS**: `use crate::services::metadata; use crate::models::work;`
- **GOTCHA**: Extraction is CPU-bound (just struct mapping, very fast) — can run inline without spawn_blocking. The DB calls are async and already in an async context.
- **GOTCHA**: The `pub_date` column is `DATE` type — bind as `Option<time::Date>`, not string.
- **GOTCHA**: `std::fs::rename` only works within the same filesystem. Library path is always a single directory tree, so this is safe. Wrap in a match and log on failure.
- **GOTCHA**: The rename must happen in spawn_blocking since it's filesystem I/O that could block.
- **VALIDATE**: Existing integration test `scan_once_processes_epub_end_to_end` should still pass (minimal EPUB has no metadata → falls through to heuristic path). Add new test: ingest EPUB with metadata "J. R. R. Tolkien - The Hobbit.epub" containing OPF author "J. R. R. Tolkien" and title "The Hobbit" → file ends up at `Tolkien, J. R. R./The Hobbit.epub`, manifestation.file_path matches. Ingest plain PDF → heuristic path unchanged.

### Task 10: Wire up module structure

- **ACTION**: Create `services/metadata/mod.rs` and register in `services/mod.rs` and `models/mod.rs`
- **IMPLEMENT**:
  `backend/src/services/metadata/mod.rs`:
  ```rust
  pub mod draft;
  pub mod extractor;
  pub mod inversion;
  pub mod isbn;
  pub mod sanitiser;
  ```
  Update `backend/src/services/mod.rs`:
  ```rust
  pub mod epub;
  pub mod ingestion;
  pub mod metadata;
  ```
  Update `backend/src/models/mod.rs`:
  ```rust
  pub mod device_token;
  pub mod ingestion_job;
  pub mod user;
  pub mod work;
  ```
- **MIRROR**: Existing mod.rs patterns (docstring + pub mod declarations)
- **IMPORTS**: N/A
- **GOTCHA**: Do this early (after Task 2) so that subsequent tasks can compile incrementally.
- **VALIDATE**: `cargo check` passes with empty module files (just `// TODO` placeholders).

---

## Testing Strategy

### Unit Tests

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| isbn10_valid | "0-306-40615-2" | valid=true, isbn_10="0306406152" | No |
| isbn10_with_x | "0-8044-2957-X" | valid=true, isbn_10="080442957X" | Yes |
| isbn13_valid | "978-0-306-40615-7" | valid=true, isbn_13="9780306406157" | No |
| isbn_invalid_checksum | "978-0-306-40615-0" | valid=false | No |
| isbn10_to_13 | "0306406152" | "9780306406157" | No |
| isbn_from_urn | "urn:isbn:9780306406157" | valid=true, isbn_13="9780306406157" | Yes |
| isbn_non_isbn | "urn:uuid:12345" | valid=false | Yes |
| isbn_empty | "" | valid=false | Yes |
| sanitise_html | "<p>Hello <em>world</em></p>" | "Hello world" | No |
| sanitise_entities | "Smith &amp; Jones" | "Smith & Jones" | No |
| sanitise_word_markup | "<o:p>text</o:p>" | "text" | Yes |
| sanitise_cdata | "<![CDATA[text]]>" | "text" | Yes |
| inversion_detected | title="Smith, John", authors=[] | Some(InversionResult) | No |
| inversion_not_title | title="Murder, She Wrote", authors=[] | None | Yes |
| inversion_normal | title="The Hobbit", authors=["Tolkien"] | None | No |
| extract_full_opf | OpfData with all fields | All ExtractedMetadata fields populated | No |
| extract_minimal_opf | OpfData with empty metadata | All fields None/empty | Yes |
| extract_date_formats | "2020", "2020-01", "2020-01-15" | Correct time::Date for each | Yes |
| opf_dc_elements_parsed | OPF XML with DC namespace | OpfData.title, creators populated | No |
| opf_calibre_series | OPF with calibre:series meta | OpfData.series_meta populated | No |
| opf_epub3_collection | OPF with belongs-to-collection | OpfData.series_meta populated | No |

### Integration Tests (require PostgreSQL)

| Test | Description |
|---|---|
| work_find_or_create_new | No existing work → creates Work + Author + work_authors |
| work_find_by_isbn | Existing manifestation with same ISBN → returns existing work_id |
| work_find_by_similarity | Existing work with similar title+author → returns match |
| work_no_false_positive | Different title → creates new work |
| draft_write_and_read | Write drafts → query metadata_versions → verify rows |
| scan_once_epub_with_metadata | Ingest metadata-rich EPUB → verify work has real title, author exists, metadata_versions populated |
| scan_once_path_rename | Ingest EPUB whose metadata differs from filename → file renamed to metadata-based path |

### Edge Cases Checklist

- [x] Empty/missing metadata fields (no title, no author)
- [x] Invalid ISBN checksums
- [x] Non-ISBN identifiers (UUIDs, URNs)
- [x] HTML in description fields
- [x] Multi-author EPUBs
- [x] Title-author inversion
- [x] Calibre vs EPUB 3 series metadata
- [x] Date in various formats (YYYY, YYYY-MM, YYYY-MM-DD)
- [x] Non-EPUB formats (PDF) bypass metadata extraction
- [x] Concurrent ingestion of same book (transaction safety)
- [x] Path collision after metadata rename

---

## Validation Commands

### Static Analysis
```bash
cd backend && cargo clippy -- -D warnings
```
EXPECT: Zero warnings

### Unit Tests
```bash
cd backend && cargo test -- --skip ignored
```
EXPECT: All new + existing unit tests pass

### Integration Tests
```bash
cd backend && cargo test -- --ignored
```
EXPECT: All integration tests pass (requires PostgreSQL with migrations applied)

### Full Build
```bash
cd backend && cargo build
```
EXPECT: Clean compilation, no warnings

### Database Validation
```bash
cd backend && DATABASE_URL=postgres://tome_ingestion:tome_ingestion@localhost:5433/tome_dev sqlx migrate run
```
EXPECT: No new migrations needed (Step 6 uses existing schema)

---

## Acceptance Criteria

- [ ] Dublin Core fields (title, creator, description, publisher, date, language, identifier, subject) extracted from EPUB 2 and EPUB 3 OPF variants
- [ ] ISBN-10 and ISBN-13 checksum validation works (valid passes, invalid flagged)
- [ ] ISBN-10 → ISBN-13 conversion implemented
- [ ] Title-author inversion detected on comma-pattern cases
- [ ] HTML stripped from descriptions, whitespace normalised, entities decoded
- [ ] Series metadata extracted from both calibre and EPUB 3 collection patterns
- [ ] All extracted metadata stored as draft `metadata_version` rows (source=opf, status=draft)
- [ ] Work deduplication: ISBN match first, then pg_trgm title+author similarity (threshold 0.6)
- [ ] Author records created with correct sort_name and role
- [ ] Series records created and linked via series_works
- [ ] Path template re-renders with extracted metadata; file atomically renamed if path changed
- [ ] Non-EPUB formats fall back to existing filename-heuristic behavior
- [ ] No new migrations required — uses existing schema
- [ ] Confidence score computed based on field completeness

## Completion Checklist

- [ ] Code follows discovered patterns (sqlx::FromRow, thiserror, tracing structured fields)
- [ ] Error handling: extraction failures are logged and gracefully degraded, never crash the pipeline
- [ ] Logging follows tracing pattern with structured fields (title, author, isbn, confidence)
- [ ] Tests follow test patterns (#[cfg(test)] inline, #[ignore] for DB tests, cleanup_test_data)
- [ ] No hardcoded values (similarity threshold could move to config later, but 0.6 default is fine)
- [ ] No unnecessary scope additions (no enrichment API, no UI, no writeback)
- [ ] Self-contained — no questions needed during implementation

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| pg_trgm similarity threshold too aggressive (false matches) | Medium | High | Conservative 0.6 threshold; all matches are draft-only (human can reject via Step 10 UI) |
| OPF namespace handling differences between EPUB 2/3 | Medium | Medium | Match both prefixed (`dc:title`) and unprefixed (`title`) element names |
| Title-author inversion false positives | Low | Low | Detection is advisory only — stored as draft, never auto-applied |
| Transaction deadlock on concurrent ingestion | Low | Medium | Short transactions, consistent lock ordering (work → author → manifestation) |
| Path rename race condition | Low | Medium | Rename within library_path (same FS), wrapped in transaction with DB update |

## Notes

- **No new dependencies needed.** quick-xml handles OPF parsing, serde_json handles JSONB, time crate handles dates. ISBN validation and HTML stripping are pure Rust.
- **No new migrations needed.** All tables and columns exist from Steps 1 and 5. The `metadata_source` enum already has 'opf' value. The `author_role` enum has all needed values.
- **The orchestrator CTE rewrite is the highest-risk change.** The current single-CTE insert becomes a multi-step transactional flow. Test thoroughly with the existing integration tests before adding new ones.
- **Author deduplication is intentionally deferred.** The authors table has no unique constraint. Step 6 creates new Author records even if the name already exists. Step 7 (enrichment) is the right place to merge duplicates using external data. For now, the work_authors join correctly links each work to its authors.
- **The `opf:role` → `author_role` mapping covers the common cases.** Roles not in the enum default to "author". MARC relator codes beyond aut/edt/trl/nrt are mapped to "author" for MVP.
