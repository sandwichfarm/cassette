# Repository Guidelines

## Project Structure & Module Organization
- `cli/` — Rust CLI (binary `cassette`); primary entrypoint.
- `cassette-tools/` — Rust library with WASM interface and modular NIP support.
- `cassette-core/` — Core traits/helpers for standardized cassette exports.
- `bindings/` — Language loaders (`js/`, `py/`, `rust/`, `go/`, `cpp/`, `dart/`).
- `gui/` — Svelte demo for local testing; `site/` — marketing/docs site.
- `tests/` — Node + shell integration/E2E tests; `cassettes/` — built artifacts.

## Build, Test, and Development Commands
- Build CLI: `make build` (debug) | `make release` (optimized).
- Test Rust crates: `make test` (or `cd cli && cargo test`, `cd cassette-tools && cargo test`).
- Lint/format: `make lint` (clippy) | `make fmt` (rustfmt) | `make check` (lint+test).
- Example + run relay: `make example` (creates `.wasm` in `cli/cassettes/`) → `make listen`.
- GUI/site dev: `make serve-gui` (from `gui/`) | `make serve-site` (from `site/`).
- Docker: `docker-compose up -d` (exposes `8080`; configure with `.env`, see `.env.sample`).
- WASM toolchain: `make wasm-target` or `rustup target add wasm32-unknown-unknown`.

## Coding Style & Naming Conventions
- Rust: run `make fmt` and `make lint` before pushing. Snake_case for functions, UpperCamelCase for types, kebab-case crates/binaries. Avoid panics in user paths; return `Result`.
- JS/TS (bindings/gui): ESM modules, explicit types where possible. Keep loader interface stable (`scrub`, `info`, `set_info`, memory helpers).
- Cassette filenames: kebab-case with `.wasm` (CLI sanitizes names).

## Testing Guidelines
- Rust unit/integration tests live in `cli/` and `cassette-tools/` (`cargo test`).
- JS/E2E: Node 18+; run scripts in `tests/` (e.g., `node tests/test-simple.js`) and shell suites (e.g., `tests/integration-test.sh`).
- Add tests with changes; prefer unit tests in `cassette-tools`, CLI integration in `cli`, and end-to-end flows in `tests`.

## Commit & Pull Request Guidelines
- Commits: short, imperative, and scoped (e.g., `cli: handle COUNT filters`, `docs: update README`).
- PRs: include description, rationale, reproduction/usage steps, and screenshots for GUI changes. Link issues, update docs/README when behavior changes. Run `make check` before submitting.

## Security & Configuration Tips
- Never commit secrets; use `cp .env.sample .env` and set `HOST_CASSETTE_DIR`, `CASSETTE_DIR`, `RUST_LOG` for Docker.
- Default relay port is `8080`. Containers run non-root by default.
- WASM interface is alpha; avoid breaking exported function names/signatures and memory conventions.

## Architecture Overview
- Components: `cli/` (Rust CLI + relay server via WebSocket, Wasmtime for WASM), `cassette-tools/` (WASM interface + NIP modules/feature flags), `cassette-core/` (common traits/helpers), `bindings/` (language loaders), `gui/` and `site/` (UX/docs).
- Data flow: record → compile `.wasm` cassette; scrub/listen → host calls into cassette; deck → writable relay that rotates, compiles, and hot‑loads cassettes.
- Stable exports: `scrub()` (loops REQ until EOSE; array result), `info()`, `set_info()`, plus memory helpers (`alloc_buffer`, `dealloc_string`). Keep names and semantics stable across crates and bindings.

## Agent-Specific Notes
- When changing WASM interface or NIP behavior, synchronize updates in `cassette-tools/`, `cli/` (commands, deck), `bindings/js` types/tests, and `tests/` Node + shell suites. Update `README.md` examples accordingly.
- Prefer adding capabilities behind `cassette-tools` feature flags (`nip11`, `nip45`, `nip50`, etc.). Maintain backward compatibility for `scrub/info/set_info` and memory conventions.
- Validate locally: `make check` (lint+tests), targeted runs like `cd cli && cargo test`, `cd cassette-tools && cargo test`, and relevant `tests/*.sh` or `node tests/*.js`.
- For CLI-visible changes, keep output stable and kebab-case flags; update help text and docs. Ensure cassette filenames remain sanitized (kebab-case `.wasm`).
