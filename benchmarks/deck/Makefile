# Deck Benchmark Makefile

PYTHON = python3
BENCHMARK_SCRIPT = deck_benchmark.py
DECK_URL = ws://localhost:9999
DURATION = 60
CONNECTIONS = 10
RPS = 100

.PHONY: all run install-deps clean venv

all: run

run: venv
	. venv/bin/activate && $(PYTHON) $(BENCHMARK_SCRIPT) --url $(DECK_URL) --duration $(DURATION) --connections $(CONNECTIONS) --rps $(RPS)

venv:
	$(PYTHON) -m venv venv
	. venv/bin/activate && pip install -r requirements.txt

requirements.txt:
	echo "# Deck benchmark requirements" > requirements.txt
	echo "websockets" >> requirements.txt
	echo "asyncio" >> requirements.txt

install-deps: venv

clean:
	rm -rf venv
	rm -f results/*.json
	find . -type d -name __pycache__ -exec rm -rf {} +