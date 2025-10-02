# Testing Documentation

This document describes the test structure and how to run tests in the Rusty Golf project.

## Test Commands

- All tests: `cargo test`
- Single test file: `cargo test --test test1`
- Single test function: `cargo test test_name`
- Show output: `cargo test -- --nocapture`
- SQL middleware tests: `cd sql-middleware && cargo test test_name`

## Test Structure

The project uses integration tests located in the `tests/` directory. Each test focuses on a specific aspect of the application:

### Test 1: Scores Endpoint (`test1_test_scores.rs`)
- **Purpose**: Tests the main scores HTTP endpoint
- **Files**: 
  - `test1.sql` - SQL setup script
  - `test1_expected_output.json` - Expected JSON output
  - `test1_test_scores.rs` - Test implementation
- **What it tests**: Score retrieval and HTTP response formatting

### Test 2: SQL Queries (`test2.sql`)
- **Purpose**: Standalone SQL query testing
- **Files**: `test2.sql` - SQL test queries

### Test 3: SQL Trait Functions (`test3_sql_trait_fns.rs`)
- **Purpose**: Tests the SQL middleware trait functions
- **Files**:
  - `test3_espn_json_responses.json` - Mock ESPN API responses
  - `test3_sql_trait_fns.rs` - SQL middleware tests
- **What it tests**: Database abstraction layer functionality

### Test 4: Cache (`test4_cache.rs`)
- **Purpose**: Tests caching functionality
- **Files**: `test4_cache.rs` - Cache implementation tests
- **What it tests**: ESPN API response caching

### Test 5: Database Prefill (`test5_dbprefill.rs`)
- **Purpose**: Tests database population from JSON configuration
- **Files**:
  - `test5_dbprefill.json` - Sample tournament data
  - `test5_dbprefill.rs` - Database prefill tests
- **What it tests**: JSON-based database initialization

### Test 6: Bar Width Rendering (`test6_bar_width.rs`)
- **Purpose**: Tests HTML template rendering and bar width calculations
- **Files**:
  - `test6/test6_ref_html.html` - Reference HTML output
  - `test6/debug/actual_output.html` - Generated output for debugging
  - `test6_bar_width.rs` - Template rendering tests
- **What it tests**: Visual scoreboard rendering accuracy

### Test 7: Step Factor Rendering (`test7_new_step_factor.rs`)
- **Purpose**: Tests score display step factor calculations
- **Files**:
  - `test7/test7_dbprefill.json` - Tournament configuration
  - `test7/detailed_scores_*.json` - Score data for multiple events
  - `test7/summary_scores_x_*.json` - Summary score data
  - `test7/reference_html_*.html` - Expected HTML output
  - `test7/debug_*/actual_output.html` - Debug output
  - `test7_new_step_factor.rs` - Step factor tests
- **What it tests**: Score visualization scaling and rendering

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
cargo test --test test1

# Run a specific test function
cargo test test_dbprefill

# Run with debug output
cargo test test_dbprefill -- --nocapture
```

## Test Data Files

- **JSON Files**: Mock API responses, tournament configurations, and expected outputs
- **SQL Files**: Database schema and test queries  
- **HTML Files**: Reference outputs for template rendering validation

## Debug Output

Several tests generate debug HTML files in `tests/test*/debug/` directories to help visualize rendering differences when tests fail.