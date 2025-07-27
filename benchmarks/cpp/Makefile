# C++ Cassette Benchmark Makefile

CXX = g++
CXXFLAGS = -std=c++17 -O3 -Wall -I../../bindings/cpp/include
LDFLAGS = -lwasmtime -lnlohmann_json

# Source files
SOURCES = benchmark.cpp ../../bindings/cpp/src/cassette_loader.cpp
OBJECTS = $(SOURCES:.cpp=.o)
TARGET = benchmark

# Benchmark samples directory
SAMPLES_DIR = ../samples

# Default cassette files to benchmark
CASSETTES = $(SAMPLES_DIR)/small.wasm $(SAMPLES_DIR)/medium.wasm $(SAMPLES_DIR)/benchmark-large.wasm

.PHONY: all clean run install-deps

all: $(TARGET)

$(TARGET): $(OBJECTS)
	$(CXX) $(OBJECTS) -o $(TARGET) $(LDFLAGS)

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c $< -o $@

run: $(TARGET)
	./$(TARGET) $(CASSETTES)

install-deps:
	@echo "Installing C++ dependencies..."
	@echo "Please ensure you have:"
	@echo "  - wasmtime C++ bindings installed"
	@echo "  - nlohmann/json library installed"
	@echo ""
	@echo "On macOS with Homebrew:"
	@echo "  brew install wasmtime nlohmann-json"
	@echo ""
	@echo "On Ubuntu/Debian:"
	@echo "  apt-get install libnlohmann-json3-dev"
	@echo "  # Install wasmtime from https://wasmtime.dev/"

clean:
	rm -f $(TARGET) $(OBJECTS) benchmark_cpp_*.json