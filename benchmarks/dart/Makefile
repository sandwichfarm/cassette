# Dart Cassette Benchmark Makefile

DART = dart
SAMPLES_DIR = ../samples
CASSETTES = $(SAMPLES_DIR)/small.wasm $(SAMPLES_DIR)/medium.wasm $(SAMPLES_DIR)/benchmark-large.wasm
ITERATIONS = 100

.PHONY: all run install-deps clean

all: run

run: install-deps
	$(DART) run benchmark.dart $(CASSETTES) --iterations $(ITERATIONS)

install-deps:
	$(DART) pub get

clean:
	rm -rf .dart_tool/
	rm -f benchmark_dart_*.json
	rm -f pubspec.lock