# Rusty-Golf Commands and Style Guide

## Build & Run Commands
- Build: `cargo build`
- Run: `cargo run`
- Check: `cargo clippy`
- Format: `cargo fmt`
- Docker: `make build`, `make clean`, `make rebuild`

## Test Commands
- All tests: `cargo test`
- Single test file: `cargo test --test test1`
- Single test function: `cargo test test_name`
- Show output: `cargo test -- --nocapture`
- SQL middleware tests: `cd sql-middleware && cargo test test_name`

## Code Style Guidelines
- Rust 2021 edition with Tokio async runtime
- Use snake_case for variables/functions, PascalCase for types/traits
- Group imports: std lib, external crates, local modules
- Error handling: `Result<T, Box<dyn std::error::Error>>` or custom error types
- SQL middleware: Use typed queries with `QueryAndParams`, proper error handling
- Database: Use `sql-middleware` abstractions for consistent DB operations
- Documentation: Include SQL file paths in test code, use descriptive names
- Run `cargo fmt` and `cargo clippy` before committing

## SQL Middleware Usage
- Database connections: `ConfigAndPool::new_postgres/sqlite`
- Queries: Use `convert_sql_params` for parameter conversions
- Error handling: Use `SqlMiddlewareDbError` and propagate with `?`
- Consistent API between database backends