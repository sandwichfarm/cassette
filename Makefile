.PHONY: test start-services test-filters test-with-services help build-wasm

# Default target
help:
	@echo "Available targets:"
	@echo "  make test             - Run all integration tests (starts services if needed)"
	@echo "  make start-services   - Start boombox and nostr-proxy services"
	@echo "  make test-filters     - Run filter tests only (assumes services are running)"
	@echo "  make build-wasm       - Build WebAssembly modules for the project"
	@echo "  make clean            - Stop running services and clean up logs"
	@echo "  make help             - Show this help message"

# Run all integration tests
test:
	@echo "Running integration tests..."
	@bash ./tests/integration-test.sh

# Start services only
start-services:
	@echo "Starting services only..."
	@bash ./tests/integration-test.sh --no-tests
 
# Run filter tests (assumes services are running)
test-filters:
	@echo "Running filter tests only..."
	@cd tests && NODE_PATH=.. node test-nip119.js

# Build WebAssembly modules
build-wasm:
	@echo "Building WebAssembly modules..."
	@cd boombox && bun run build:wasm

# Clean up
clean:
	@echo "Stopping services and cleaning up..."
	@pkill -f 'bun run' || true
	@rm -rf logs/*.log
	@echo "Services stopped and logs cleaned" 