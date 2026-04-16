# Implementation Report: OPF Metadata Extraction and ISBN Validation

## Summary
Implemented Dublin Core metadata extraction from EPUB OPF files during ingestion. The pipeline now extracts title, creators, description, publisher, date, language, identifiers (with ISBN validation), subjects, and series metadata. Extracted fields are stored as auditable draft `metadata_version` rows. The previous blind `INSERT INTO works` CTE has been replaced with intelligent work-matching (ISBN lookup, then pg_trgm fuzzy title+author similarity) and proper author/series record creation. Files are atomically renamed to metadata-based paths before the DB insert.

## Assessment vs Reality

| Metric | Predicted (Plan) | Actual |
|---|---|---|
| Complexity | Large | Large |
| Confidence | 8/10 | 9/10 |
| Files Changed | 12-15 | 13 (8 new, 5 modified) |

## Tasks Completed

| # | Task | Status | Notes |
|---|---|---|---|
| 10 | Wire up module structure | Complete | Done first to enable incremental compilation |
| 1 | Extend OpfData with Dublin Core fields | Complete | 7 new tests |
| 2 | Carry OpfData through ValidationReport | Complete | 5 return paths updated |
| 3 | Create ISBN validation module | Complete | 11 unit tests |
| 4 | Create metadata sanitiser | Complete | 12 unit tests |
| 5 | Create title-author inversion detector | Complete | 6 unit tests |
| 6 | Create metadata extractor | Complete | 6 unit tests |
| 7 | Create draft metadata writer | Complete | No unit tests (DB-only) |
| 8 | Create work matching model | Complete | No unit tests (DB-only) |
| 9 | Rewrite orchestrator CTE | Complete | Metadata-aware flow with path rename |

## Validation Results

| Level | Status | Notes |
|---|---|---|
| Clippy | Pass | Zero warnings with -D warnings |
| Formatting | Pass | cargo fmt clean |
| Unit Tests | Pass | 113 total (42 new) |
| Build | Pass | Zero warnings |

## Files Changed

| File | Action | Purpose |
|---|---|---|
| `backend/src/services/epub/opf_layer.rs` | UPDATED | Extended XML loop with DC extraction, Creator/SeriesMeta structs |
| `backend/src/services/epub/mod.rs` | UPDATED | Added opf_data field to ValidationReport |
| `backend/src/services/epub/cover_layer.rs` | UPDATED | Updated test OpfData construction for new fields |
| `backend/src/services/metadata/mod.rs` | CREATED | Module root with re-exports |
| `backend/src/services/metadata/isbn.rs` | CREATED | ISBN-10/13 validation and conversion |
| `backend/src/services/metadata/sanitiser.rs` | CREATED | HTML stripping, entity decoding, whitespace normalisation |
| `backend/src/services/metadata/inversion.rs` | CREATED | Title-author inversion heuristic |
| `backend/src/services/metadata/extractor.rs` | CREATED | OpfData -> ExtractedMetadata transformation |
| `backend/src/services/metadata/draft.rs` | CREATED | metadata_version row writer |
| `backend/src/services/mod.rs` | UPDATED | Added pub mod metadata |
| `backend/src/models/work.rs` | CREATED | Work matching (ISBN/trgm) and record creation |
| `backend/src/models/mod.rs` | UPDATED | Added pub mod work |
| `backend/src/services/ingestion/orchestrator.rs` | UPDATED | Replaced CTE with metadata-aware pipeline |

## Deviations from Plan
- Entity decoding in OPF test: quick-xml does not auto-decode XML entities (`&amp;` stays as `&amp;`), so the sanitiser handles this instead. Test expectation adjusted.
- EPUB 3 group-position ordering: added comment noting best-effort behavior when elements appear out of order.

## Issues Encountered
- `br#"..."#` raw string conflicts with `#c01` in test XML — resolved with `br##"..."##` double-hash delimiter.
- 12 clippy lints (collapsible ifs, is_multiple_of, unnecessary casts) — all fixed.

## Tests Written

| Test Area | Tests | Coverage |
|---|---|---|
| opf_layer DC extraction | 5 new | title, creators, series (calibre + EPUB3), empty metadata, multi-author |
| ISBN validation | 11 | ISBN-10, ISBN-13, X check digit, conversions, URN parsing, edge cases |
| Sanitiser | 12 | HTML strip, entities, CDATA, Word markup, whitespace, pipelines |
| Inversion detection | 6 | Detected cases, false positives, normal titles, edge cases |
| Metadata extractor | 6 | Full/minimal extraction, date formats, sort names, role mapping, multi-author |
| Existing tests | 2 updated | cover_layer OpfData construction |

## Next Steps
- [ ] Code review via `/code-review`
- [ ] Create PR via `/prp-pr`
