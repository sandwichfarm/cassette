#!/usr/bin/env python3
"""Test script to debug cassette loading"""

import sys
from wasmtime import Store, Module, Instance, Memory, MemoryType, Limits

def inspect_wasm(wasm_path):
    """Inspect WASM module exports and imports"""
    with open(wasm_path, 'rb') as f:
        wasm_bytes = f.read()
    
    store = Store()
    module = Module(store.engine, wasm_bytes)
    
    print("=== WASM Module Inspection ===")
    print(f"File: {wasm_path}")
    
    print("\n📤 Exports:")
    for export in module.exports:
        print(f"  - {export.name}: {export.type}")
    
    print("\n📥 Imports:")
    for import_ in module.imports:
        print(f"  - {import_.module}.{import_.name}: {import_.type}")
    
    # Try to instantiate with memory
    print("\n🔧 Attempting instantiation...")
    
    # Check if memory is imported
    needs_memory = any(imp.name == 'memory' for imp in module.imports)
    
    if needs_memory:
        print("  Module requires imported memory")
        memory = Memory(store, MemoryType(Limits(1, None)))
        imports = [memory]
    else:
        print("  Module exports its own memory")
        imports = []
    
    try:
        instance = Instance(store, module, imports)
        print("  ✅ Instantiation successful")
        
        # Check exports
        exports = instance.exports(store)
        print("\n  Available functions:")
        for item in dir(exports):
            if not item.startswith('_'):
                print(f"    - {item}")
                
    except Exception as e:
        print(f"  ❌ Instantiation failed: {e}")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python test_loader.py <wasm_file>")
        sys.exit(1)
    
    inspect_wasm(sys.argv[1])