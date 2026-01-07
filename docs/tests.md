# Testing Documentation

This document describes tests in the project.

For information about test coverage gaps and missing tests, see [Test Coverage TODO](tests_todo.md).

## Test Commands

- All tests (preferred): `cargo nextest run --no-fail-fast`
- Single test binary: `cargo nextest run --bin test01_test_scores`
- All with cargo (fallback): `cargo test`
- Show output (cargo): `cargo test -- --nocapture`

## Test Structure

The project uses integration tests located in the `tests/` directory. Each test focuses on a specific aspect of the application:

### Test 1: Scores Endpoint
- **Purpose**: Tests the main scores HTTP endpoint. Talks to ESPN and compares results to `test01_expected_output.json`
- **Files**: 
  - `test01.sql` - SQL setup script
  - `test01_expected_output.json` - Expected results
  - `test01_test_scores.rs` - the test
- **Database**: `file::memory:?cache=shared".to_string();`
- **What it tests**: Score retrieval and json response format

### Test 3: SQL Trait Functions
- **Purpose**: Tests `get_data_for_scores_page`
- **Files**:
  - `test03_espn_json_responses.json` - Mock `scores.rs` API responses
  - `test03_sql_trait_fns.rs` - the test
  - `test01.sql` - Used to load the database
- **Database**: `file::memory:?cache=shared".to_string();`
- **What it tests**: Database abstraction layer functionality and json score formatting
- **Optional R2 extension**: If a `.env` file exists with `R2_ENDPOINT`, `R2_BUCKET`, `R2_ACCESS_KEY_ID`, and `R2_SECRET_ACCESS_KEY`, the test also writes scores to R2, reads them back, and renders HTML from the R2 payload to validate parity with SQLite.

### Test 4: Cache
- **Purpose**: Tests caching functionality
- **Files**: 
  - `test04_cache.rs` - the test
  - `test01.sql` - used to populate the db
- **Database**: `file::memory:?cache=shared".to_string();`
- **What it tests**: `score_data.last_refresh` formatting

### Test 5: Database Prefill
- **Purpose**: Tests database population from JSON configuration
- **Files**:
  - `test05_dbprefill.json` - Sample tournament data
  - `test05_dbprefill.rs` - Database prefill tests
- **Database**: `file::memory:?cache=shared".to_string();`
- **What it tests**: JSON-based database initialization `db_prefill.rs`

### Test 6: Bar Width Rendering
- **Purpose**: Tests HTML template rendering and bar width calculations
- **Files, data structures, and parts tested**:
  - `test06/test06_ref_html.html` - Reference HTML output, loaded with crate `scraper` and compared against `test_render_template()` output 
  - `test06/debug/actual_output.html` - Generated output for debugging
  - `test06_bar_width.rs` - the test
  - `test05_dbprefill.json` - Loaded test data
  - `detailed_scores` - Data structure used to make 
  - `test_render_template()` - key fn tested
- **Database**: `file::memory:?cache=shared".to_string();`
- **What it tests**: Html rendering of a complex "bar graph" we custom wrote

### Test 7: Step Factor Rendering (`test7_new_step_factor.rs`)
- **Purpose**: Tests score display step factor calculations
- **Files**:
  - `test07/test07_dbprefill.json` - Tournament configuration
  - `test07/detailed_scores_*.json` - Score data for multiple events
  - `test07/summary_scores_x_*.json` - Summary score data
  - `test07/reference_html_*.html` - Expected HTML output
  - `test07/debug_*/actual_output.html` - Debug output
  - `test07_new_step_factor.rs` - the test
- **Database**: `file::memory:?cache=shared".to_string();`
- **What it tests**: Whether score view step factor logic is functioning as designed

## Test Database Setup

Most tests use in-memory SQLite databases for isolation:
```rust
let x = "file::memory:?cache=shared".to_string();
let config_and_pool = ConfigAndPool::new_sqlite(x).await.unwrap();
```

Tests automatically create required database tables using SQL files from:
- `src/sql/schema/sqlite/` - SQLite schema files
- `src/sql/schema/postgres/` - PostgreSQL schema files

## Running Specific Tests

```bash
# Run a specific integration test
cargo test --test test01

# Run with debug output
cargo test --test test4_cache -- --nocapture
```

## Debug Output

Several tests generate debug HTML files in `tests/test*/debug/` directories to help visualize rendering differences when tests fail.

## Offline Mode (No Network)

- ESPN HTTP calls automatically fall back to a local fixture if the network is unavailable.
- Trigger: any reqwest error during fetch causes a fallback to `tests/test03_espn_json_responses.json`.
- Behavior: only the `score_struct` array is loaded and written to the DB via the normal storage path; timestamps and rendering behave as with live data.
- This keeps CI and local runs deterministic without requiring internet access.
