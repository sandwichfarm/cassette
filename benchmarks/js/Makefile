# JavaScript/Node.js Cassette Benchmark Makefile

NODE = node
NPM = npm
SAMPLES_DIR = ../samples
CASSETTES = $(SAMPLES_DIR)/small.wasm $(SAMPLES_DIR)/medium.wasm $(SAMPLES_DIR)/large.wasm

.PHONY: all run install-deps clean

all: run

run: install-deps
	$(NODE) benchmark.js $(CASSETTES)

install-deps:
	$(NPM) install

clean:
	rm -rf node_modules
	rm -f package-lock.json
	rm -f benchmark_js_*.json