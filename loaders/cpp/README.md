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
        std::string result = cassette.send(req);
        std::cout << "REQ Result: " << result << std::endl;
        
        // Send a CLOSE message
        std::string close_msg = R"(["CLOSE", "sub1"])";
        std::string close_result = cassette.send(close_msg);
        std::cout << "CLOSE Result: " << close_result << std::endl;
        
        // Send a COUNT message (NIP-45)
        std::string count_msg = R"(["COUNT", "count-sub", {"kinds": [1]}])";
        std::string count_result = cassette.send(count_msg);
        std::cout << "COUNT Result: " << count_result << std::endl;
        
        // Get relay info (NIP-11)
        try {
            std::string info = cassette.info();
            std::cout << "Relay Info: " << info << std::endl;
        } catch (const std::exception& e) {
            std::cout << "Info not available: " << e.what() << std::endl;
        }
        
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
```

## Features

- Full WebAssembly support via wasmtime C++ API
- Unified `send` method for all NIP-01 messages (v0.5.0+)
- MSGB format support for memory operations
- Event deduplication (automatically reset on new REQ messages)
- Newline-separated message handling
- Thread-safe operations
- Exception-based error handling
- Debug logging support
- Automatic synthesis of `describe()` from `info()` method

## CMake Integration

To use in your CMake project:

```cmake
find_package(cassette-loader REQUIRED)

add_executable(myapp main.cpp)
target_link_libraries(myapp cassette::cassette-loader)
```