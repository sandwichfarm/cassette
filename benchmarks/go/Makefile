# Go Cassette Benchmark Makefile

GO = go
SAMPLES_DIR = ../samples
CASSETTES = $(SAMPLES_DIR)/small.wasm $(SAMPLES_DIR)/medium.wasm $(SAMPLES_DIR)/benchmark-large.wasm
ITERATIONS = 100

.PHONY: all build run clean install-deps

all: run

build: install-deps
	$(GO) build -o benchmark benchmark.go

run: build
	./benchmark --iterations $(ITERATIONS) $(CASSETTES)

install-deps:
	$(GO) mod download
	$(GO) mod tidy

clean:
	rm -f benchmark benchmark_go_*.json
	$(GO) clean -modcache