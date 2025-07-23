cmake_minimum_required(VERSION 3.14)
project(cassette-loader-cpp VERSION 0.1.0)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Find required packages
find_package(PkgConfig REQUIRED)
pkg_check_modules(WASMTIME REQUIRED wasmtime)

# Find nlohmann_json
find_package(nlohmann_json 3.11.0 REQUIRED)

# Create library
add_library(cassette-loader
    src/cassette_loader.cpp
)

target_include_directories(cassette-loader PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>
    ${WASMTIME_INCLUDE_DIRS}
)

target_link_libraries(cassette-loader
    ${WASMTIME_LIBRARIES}
    nlohmann_json::nlohmann_json
)

target_compile_options(cassette-loader PRIVATE ${WASMTIME_CFLAGS_OTHER})

# Example executable
add_executable(cassette-example
    examples/example.cpp
)

target_link_libraries(cassette-example
    cassette-loader
)

# Installation
install(TARGETS cassette-loader
    EXPORT cassette-loader-targets
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
)

install(DIRECTORY include/
    DESTINATION include
)

install(EXPORT cassette-loader-targets
    FILE cassette-loader-config.cmake
    NAMESPACE cassette::
    DESTINATION lib/cmake/cassette-loader
)