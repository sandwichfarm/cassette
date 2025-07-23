# Cassette Loader Tests

This directory contains tests for the cassette-loader package. These tests verify that the loader correctly loads WebAssembly cassettes and implements the standardized bindings according to the NIP-01 interface.

## Test Structure

The test suite is organized into three main categories:

- **Unit Tests** (`loader.test.js`): Tests basic functionality of loading cassettes and the standardized bindings.
- **Memory Management Tests** (`memory.test.js`): Tests WASM memory interactions, string conversions, and memory allocation/deallocation.
- **Integration Tests** (`integration.test.js`): Tests the loader functionality with real cassettes and mocked responses.

## Running Tests

Before running the tests, make sure you have built the package:

```bash
# Build the package
npm run build
```

Then you can run the tests:

```bash
# Run all tests
npm test

# Run specific test categories
npm run test:unit
npm run test:memory
npm run test:integration

# Run tests with coverage report
npm run test:coverage

# Run tests in watch mode (useful during development)
npm run test:watch

# Run the legacy test
npm run test:legacy
```

## Test Fixtures

The `fixtures` directory contains files used for testing:

- `mock-cassette.js`: A mock implementation of a cassette that follows the NIP-01 interface.
- `invalid.wasm`: An invalid WASM file for testing error handling.

## Creating a Test Cassette

For integration tests, you will need a valid test cassette. You can create one using the CLI:

```bash
cd ..
cd cli
cargo run -- dub path/to/events.json --name test-cassette
```

This will create a test cassette in the `cli/cassettes` directory.

## Mocking Cassettes

For tests that don't require an actual WASM cassette, you can use the mock implementation in `fixtures/mock-cassette.js`.

## Coverage

After running `npm run test:coverage`, a coverage report will be generated in the `coverage` directory. You can open `coverage/lcov-report/index.html` in a browser to view the report.

## Common Issues

### Running Tests with Node.js ES Modules

Since the package is using ES modules, you need to run Jest with the `--experimental-vm-modules` flag. This is already configured in the npm scripts.

### WebAssembly Tests in Node.js

Some tests may not work in certain environments due to WebAssembly instantiation restrictions. Ensure you are running a recent version of Node.js (>= 14) with WebAssembly support.

### Missing Test Cassette

If the test cassette is not found, some tests will be skipped. Make sure to create a test cassette as described above before running the integration tests. 