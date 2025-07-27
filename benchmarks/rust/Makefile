# Rust Cassette Benchmark Makefile

CARGO = cargo
SAMPLES_DIR = ../samples
CASSETTES = $(SAMPLES_DIR)/small.wasm $(SAMPLES_DIR)/medium.wasm $(SAMPLES_DIR)/large.wasm

.PHONY: all build run clean install-deps

all: run

build:
	$(CARGO) build --release

run: build
	$(CARGO) run --release -- $(CASSETTES)

install-deps:
	@echo "Rust dependencies are managed by Cargo"
	@echo "Run 'cargo build' to install dependencies"

clean:
	$(CARGO) clean
	rm -f benchmark_*.json