# Changelog

## March 2025 Updates

### Added
- **NIP-119 Support**: Implemented AND tag filtering using the '&' prefix for tag keys 
  - Added ability to match events with ALL specified tag values for a given tag type
  - Added test script `test-nip119.js` for verifying NIP-119 functionality
  - Added specific test case with t-tags 'value1' and 'value2'

### Improved
- **Integration Tests**: Enhanced the integration test script with comprehensive filter testing
  - Added automated testing for all supported filters
  - Added '--no-tests' flag option to start servers without running tests
  - Fixed command syntax to use simplified nak parameters (-i, -p instead of JSON)

### Documentation
- **README Updates**: Expanded documentation to include:
  - NIP-119 implementation details
  - Examples of using AND tag filtering
  - Improved integration test instructions
  - Updated command examples to match the latest syntax

### Fixed
- Fixed filter parsing in the Rust implementation
- Fixed signature verification issues in test data
- Improved WebAssembly binding and cassette loading 