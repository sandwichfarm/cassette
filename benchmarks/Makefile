# Top-level Makefile for Cassette Benchmarks

.PHONY: all run setup install-deps clean help py rust cpp dart go js deck

# Default target
all: setup run

# Generate test cassettes for benchmarks
setup:
	@echo "🔧 Setting up benchmark test data..."
	@echo ""
	@# Check if cassettes already exist
	@if [ -d "samples" ] && [ "$$(ls -A samples 2>/dev/null)" ]; then \
		echo "⚠️  Cassettes already exist in samples/"; \
		echo "   Run 'make clean-cassettes' to remove them first if you want to regenerate"; \
		echo ""; \
		echo "📊 Existing cassettes:"; \
		ls -la samples/*.wasm 2>/dev/null | wc -l | xargs echo "   Total cassettes:"; \
		du -sh samples 2>/dev/null | awk '{print "   Total size: " $$1}'; \
	else \
		echo "📦 Generating benchmark cassettes..."; \
		chmod +x scripts/generate_samples.sh 2>/dev/null || true; \
		if [ -x "scripts/generate_samples.sh" ]; then \
			./scripts/generate_samples.sh; \
		else \
			echo "❌ No cassette generation script found!"; \
			echo "   Please ensure scripts/generate_benchmark_suite.sh exists"; \
			exit 1; \
		fi; \
		echo ""; \
		echo "✅ Cassette generation complete!"; \
	fi
	@echo ""

# Run all benchmarks
run:
	@echo "🚀 Running all Cassette benchmarks..."
	@./run_all_benchmarks.sh

# Install dependencies for all languages
install-deps:
	@echo "📦 Installing dependencies for all languages..."
	@echo ""
	@echo "➤ Rust dependencies..."
	@$(MAKE) -C rust install-deps
	@echo ""
	@echo "➤ Python dependencies..."
	@$(MAKE) -C py install-deps
	@echo ""
	@echo "➤ C++ dependencies..."
	@$(MAKE) -C cpp install-deps
	@echo ""
	@echo "➤ Dart dependencies..."
	@$(MAKE) -C dart install-deps
	@echo ""
	@echo "➤ Go dependencies..."
	@$(MAKE) -C go install-deps
	@echo ""
	@echo "➤ JavaScript dependencies..."
	@$(MAKE) -C js install-deps
	@echo ""
	@echo "➤ Deck dependencies..."
	@$(MAKE) -C deck install-deps
	@echo ""
	@echo "✅ All dependencies installed!"

# Clean all benchmark artifacts
clean:
	@echo "🧹 Cleaning all benchmark artifacts..."
	@$(MAKE) -C py clean
	@$(MAKE) -C rust clean
	@$(MAKE) -C cpp clean
	@$(MAKE) -C dart clean
	@$(MAKE) -C go clean
	@$(MAKE) -C js clean
	@$(MAKE) -C deck clean
	@rm -rf results/*.json results/*.log results/*.txt
	@echo "✅ Clean complete!"

# Clean only cassettes
clean-cassettes:
	@echo "🧹 Cleaning cassette files..."
	@rm -rf samples/
	@echo "✅ Cassettes cleaned!"

# Individual language targets
py:
	@echo "🐍 Running Python benchmark..."
	@$(MAKE) -C py run

rust:
	@echo "🦀 Running Rust benchmark..."
	@$(MAKE) -C rust run

cpp:
	@echo "⚙️  Running C++ benchmark..."
	@$(MAKE) -C cpp run

dart:
	@echo "🎯 Running Dart benchmark..."
	@$(MAKE) -C dart run

go:
	@echo "🐹 Running Go benchmark..."
	@$(MAKE) -C go run

js:
	@echo "📜 Running JavaScript benchmark..."
	@$(MAKE) -C js run

deck:
	@echo "🃏 Running Deck benchmark..."
	@$(MAKE) -C deck run

# Help target
help:
	@echo "Cassette Benchmarks - Available targets:"
	@echo ""
	@echo "  make setup        - Generate test cassettes for benchmarks"
	@echo "  make run          - Run all benchmarks"
	@echo "  make all          - Setup and run benchmarks (default)"
	@echo "  make install-deps - Install dependencies for all languages"
	@echo "  make clean        - Clean all benchmark artifacts"
	@echo "  make clean-cassettes - Clean only cassette files"
	@echo ""
	@echo "Individual language benchmarks:"
	@echo "  make py          - Run Python benchmark only"
	@echo "  make rust        - Run Rust benchmark only"
	@echo "  make cpp         - Run C++ benchmark only"
	@echo "  make dart        - Run Dart benchmark only"
	@echo "  make go          - Run Go benchmark only"
	@echo "  make js          - Run JavaScript benchmark only"
	@echo "  make deck        - Run Deck benchmark only"
	@echo ""
	@echo "For more options, see benchmarks/README.md"