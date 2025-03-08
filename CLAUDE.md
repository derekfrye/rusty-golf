# Rusty-Golf Commands and Style Guide

## Build & Run Commands
- Build: `cargo build`
- Run: `cargo run`
- Build Docker image: `make build`
- Clean build context: `make clean`
- Rebuild Docker image: `make rebuild`

## Test Commands
- Run all tests: `cargo test`
- Run specific test file: `cargo test --test test1`
- Run specific test function: `cargo test sqlite_multiple_column_test`
- Test with output: `cargo test -- --nocapture`
- SQL middleware tests: `cd sql-middleware && cargo test`

## Code Style Guidelines
- Use Rust 2021 edition
- Error handling: Use `Result<T, Box<dyn std::error::Error>>` for functions that can fail
- Naming: snake_case for variables/functions, PascalCase for types/traits
- Async/await: Use Tokio for async runtime
- SQL middleware: Prefer typed queries and proper error handling
- Database access: Use the `sql-middleware` crate abstractions for database operations
- Format code with `cargo fmt` before committing
- Run `cargo clippy` to check for common mistakes

## Project Structure
- `/src`: Application source code
- `/sql-middleware`: SQL abstraction middleware
- `/static`: Static assets (JS, CSS)
- `/tests`: Integration tests