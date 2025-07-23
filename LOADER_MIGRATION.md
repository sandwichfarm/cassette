# Loader Migration Summary

## Changes Made

The JavaScript and Python loaders have been moved from:
- `cassette-loader/` → `loaders/js/`
- `cassette-loader-py/` → `loaders/py/`

## Updated Files

### Package Dependencies
- ✅ `gui/package.json` - Updated to reference `../loaders/js`
- ✅ `boombox/package.json` - Updated to reference `../loaders/js`

### Import Paths
- ✅ `tests/test-cli-cassette.js` - Updated import path
- ✅ `tests/test-generated-cassette.js` - Updated import path
- ✅ `tests/test-cassette-memory.js` - Updated import path
- ✅ `tests/test-debug-cassette.js` - Updated import path

### HTML Files
- ✅ `gui/index.html` - Updated script loading paths

### Documentation
- ✅ `README.md` - Updated project structure
- ✅ `tests/README.md` - Updated references and examples
- ✅ `gui/README.md` - Updated import examples
- ✅ `cli/README.md` - Updated loader references

## Next Steps

To complete the migration, run these commands:

```bash
# Remove old node_modules and reinstall dependencies
cd gui
rm -rf node_modules package-lock.json
npm install

cd ../boombox
rm -rf node_modules bun.lockb
bun install

# Build the JavaScript loader
cd ../loaders/js
npm install
npm run build

# Run tests to verify everything works
cd ../../tests
node test-cli-cassette.js
```

## Notes

- The loader package name remains `cassette-loader` in package.json files
- Import paths have been updated to reflect the new directory structure
- Python loader paths were not updated as no files were found using them