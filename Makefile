.PHONY: help build test release install dev clean lint fix check docs

# Colors for output
GREEN := \033[0;32m
YELLOW := \033[0;33m
RED := \033[0;31m
NC := \033[0m # No Color

# Default target
help:
	@echo "$(GREEN)Cassette Development Commands$(NC)"
	@echo ""
	@echo "$(YELLOW)Build & Development:$(NC)"
	@echo "  make build       - Build CLI in debug mode"
	@echo "  make release     - Build CLI in release mode"
	@echo "  make install     - Install cassette CLI to system"
	@echo "  make wasm-target - Install wasm32 target for Rust"
	@echo ""
	@echo "$(YELLOW)Testing:$(NC)"
	@echo "  make test        - Run all tests"
	@echo "  make test-unit   - Run unit tests only"
	@echo "  make test-int    - Run integration tests"
	@echo "  make test-loader - Test language loaders"
	@echo ""
	@echo "$(YELLOW)Code Quality:$(NC)"
	@echo "  make lint        - Run clippy linter"
	@echo "  make fmt         - Format code with rustfmt"
	@echo "  make fix         - Auto-fix linting issues"
	@echo "  make check       - Run all checks (lint + test)"
	@echo ""
	@echo "$(YELLOW)Cassette Operations:$(NC)"
	@echo "  make example     - Create example cassette"
	@echo "  make listen      - Start WebSocket relay server with cassettes"
	@echo "  make clean-wasm  - Remove all .wasm files"
	@echo ""
	@echo "$(YELLOW)Development Tools:$(NC)"
	@echo "  make dev         - Run in development mode with auto-reload"
	@echo "  make docs        - Generate and open documentation"
	@echo "  make serve-gui   - Start GUI development server"
	@echo "  make serve-site  - Start website development server"
	@echo ""
	@echo "$(YELLOW)Release Management:$(NC)"
	@echo "  make version     - Show current version"
	@echo "  make bump-patch  - Bump patch version"
	@echo "  make bump-minor  - Bump minor version"
	@echo "  make bump-major  - Bump major version"

# Build commands
build:
	@echo "$(GREEN)Building Cassette CLI...$(NC)"
	@cd cli && cargo build
	@echo "$(GREEN)✓ Build complete$(NC)"

release:
	@echo "$(GREEN)Building Cassette CLI (release)...$(NC)"
	@cd cli && cargo build --release
	@echo "$(GREEN)✓ Release build complete$(NC)"

install: release
	@echo "$(GREEN)Installing Cassette CLI...$(NC)"
	@cd cli && cargo install --path .
	@echo "$(GREEN)✓ Cassette CLI installed$(NC)"

wasm-target:
	@echo "$(GREEN)Installing wasm32 target...$(NC)"
	@rustup target add wasm32-unknown-unknown
	@echo "$(GREEN)✓ WASM target installed$(NC)"

# Testing commands
test:
	@echo "$(GREEN)Running all tests...$(NC)"
	@cd cli && cargo test
	@cd cassette-tools && cargo test
	@echo "$(GREEN)✓ All tests passed$(NC)"

test-unit:
	@echo "$(GREEN)Running unit tests...$(NC)"
	@cd cli && cargo test --lib
	@cd cassette-tools && cargo test --lib
	@echo "$(GREEN)✓ Unit tests passed$(NC)"

test-int:
	@echo "$(GREEN)Running integration tests...$(NC)"
	@cd cli && cargo test --test '*'
	@echo "$(GREEN)✓ Integration tests passed$(NC)"

test-loader:
	@echo "$(GREEN)Testing language loaders...$(NC)"
	@cd loaders/js && npm test
	@cd loaders/rust && cargo test
	@cd loaders/py && python test_loader.py
	@echo "$(GREEN)✓ Loader tests passed$(NC)"

# Code quality commands
lint:
	@echo "$(GREEN)Running clippy...$(NC)"
	@cd cli && cargo clippy -- -D warnings
	@cd cassette-tools && cargo clippy -- -D warnings
	@echo "$(GREEN)✓ No linting issues$(NC)"

fmt:
	@echo "$(GREEN)Formatting code...$(NC)"
	@cd cli && cargo fmt
	@cd cassette-tools && cargo fmt
	@echo "$(GREEN)✓ Code formatted$(NC)"

fix:
	@echo "$(GREEN)Auto-fixing issues...$(NC)"
	@cd cli && cargo fix --allow-dirty --allow-staged
	@cd cli && cargo fmt
	@cd cassette-tools && cargo fix --allow-dirty --allow-staged
	@cd cassette-tools && cargo fmt
	@echo "$(GREEN)✓ Issues fixed$(NC)"

check: lint test
	@echo "$(GREEN)✓ All checks passed$(NC)"

# Cassette operations
example:
	@echo "$(GREEN)Creating example cassette...$(NC)"
	@echo '[{"id":"example123","pubkey":"79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798","created_at":1700000000,"kind":1,"tags":[],"content":"Hello from Cassette!","sig":"304402203a0c8b0ae4b2d1f6d4f8c4f8f8d8a8e8f8f8f8f8f8f8f8f8f8f8f8f8f8f8f8f8022012345678901234567890123456789012345678901234567890123456789012"}]' > example_events.json
	@cd cli && cargo run -- record ../example_events.json --name example-cassette --skip-validation
	@rm example_events.json
	@echo "$(GREEN)✓ Example cassette created at cli/cassettes/example-cassette.wasm$(NC)"

listen:
	@echo "$(GREEN)Starting Cassette relay server...$(NC)"
	@cd cli && cargo run -- listen ../cassettes/*.wasm --verbose

clean-wasm:
	@echo "$(YELLOW)Cleaning cassette files...$(NC)"
	@find . -name "*.wasm" -type f -delete
	@echo "$(GREEN)✓ All .wasm files removed$(NC)"

# Development tools
dev:
	@echo "$(GREEN)Starting development mode...$(NC)"
	@cd cli && cargo watch -x run

docs:
	@echo "$(GREEN)Generating documentation...$(NC)"
	@cd cli && cargo doc --open
	@cd cassette-tools && cargo doc --open

serve-gui:
	@echo "$(GREEN)Starting GUI development server...$(NC)"
	@cd gui && npm run dev

serve-site:
	@echo "$(GREEN)Starting website development server...$(NC)"
	@cd site && npm run dev

# Version management
version:
	@echo "$(GREEN)Current version:$(NC)"
	@grep "^version" cli/Cargo.toml | head -1 | cut -d'"' -f2

bump-patch:
	@echo "$(GREEN)Bumping patch version...$(NC)"
	@cd cli && cargo bump patch
	@echo "$(GREEN)✓ Version bumped. Don't forget to commit and tag!$(NC)"

bump-minor:
	@echo "$(GREEN)Bumping minor version...$(NC)"
	@cd cli && cargo bump minor
	@echo "$(GREEN)✓ Version bumped. Don't forget to commit and tag!$(NC)"

bump-major:
	@echo "$(GREEN)Bumping major version...$(NC)"
	@cd cli && cargo bump major
	@echo "$(GREEN)✓ Version bumped. Don't forget to commit and tag!$(NC)"

# Clean everything
clean: clean-wasm
	@echo "$(YELLOW)Cleaning build artifacts...$(NC)"
	@cd cli && cargo clean
	@cd cassette-tools && cargo clean
	@rm -rf cli/cassettes/*.wasm
	@rm -rf @cassettes
	@rm -rf logs/*.log
	@echo "$(GREEN)✓ All clean$(NC)" 