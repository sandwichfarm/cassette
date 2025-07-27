# Python Cassette Benchmark Makefile

PYTHON = python3
BENCHMARK_SCRIPT = cassette_bench_proper.py
SAMPLES_DIR = ../samples
CASSETTES = $(SAMPLES_DIR)/small.wasm $(SAMPLES_DIR)/medium.wasm $(SAMPLES_DIR)/large.wasm

.PHONY: all run install-deps clean venv

all: run

run: venv
	. venv/bin/activate && $(PYTHON) $(BENCHMARK_SCRIPT) $(CASSETTES)

venv:
	$(PYTHON) -m venv venv
	. venv/bin/activate && pip install -r requirements.txt

requirements.txt:
	echo "# Python cassette benchmark requirements" > requirements.txt
	echo "../../bindings/py/" >> requirements.txt

install-deps: venv

clean:
	rm -rf venv
	rm -f benchmark_*.json
	find . -type d -name __pycache__ -exec rm -rf {} +