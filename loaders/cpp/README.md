# Cassette C++ Loader

A C++ implementation of the Cassette loader for loading and executing Nostr event cassettes.

## Requirements

- C++17 or later
- CMake 3.14 or later
- wasmtime C API
- nlohmann/json

## Installation

### Install Dependencies

#### macOS
```bash
brew install wasmtime nlohmann-json
```

#### Ubuntu/Debian
```bash
# Install wasmtime
curl https://wasmtime.dev/install.sh -sSf | bash

# Install nlohmann-json
sudo apt-get install nlohmann-json3-dev
```

### Build

```bash
mkdir build
cd build
cmake ..
make
sudo make install
```

## Usage

```cpp
#include <cassette_loader.hpp>
#include <iostream>

int main() {
    try {
        // Load a cassette
        cassette::Cassette cassette("path/to/cassette.wasm", true);
        
        // Get cassette description
        std::string desc = cassette.describe();
        std::cout << "Description: " << desc << std::endl;
        
        // Send a REQ message
        std::string req = R"(["REQ", "sub1", {"limit": 10}])";
        
        // Loop to get all events
        while (true) {
            std::string result = cassette.send(req);
            
            if (result.empty()) {
                break;
            }
            
            std::cout << "Result: " << result << std::endl;
            
            // Check for EOSE
            if (result.find("\"EOSE\"") != std::string::npos) {
                break;
            }
        }
        
        // Send a CLOSE message
        std::string close_msg = R"(["CLOSE", "sub1"])";
        cassette.send(close_msg);
        
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
```

## Features

- Full WebAssembly support via wasmtime C++ API
- MSGB format support for memory operations
- Event deduplication
- Newline-separated message handling
- Thread-safe operations
- Exception-based error handling
- Debug logging support

## CMake Integration

To use in your CMake project:

```cmake
find_package(cassette-loader REQUIRED)

add_executable(myapp main.cpp)
target_link_libraries(myapp cassette::cassette-loader)
```