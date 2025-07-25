"""
Cassette Loader for Python
A Python implementation of the cassette loader for loading and interacting with Nostr WASM cassettes.
"""

import json
import struct
from typing import Dict, List, Optional, Tuple, Any, Union
from dataclasses import dataclass
from wasmtime import Store, Module, Instance, Memory, Func, FuncType, ValType, Limits, MemoryType
import hashlib
import time


# Constants
MSGB_SIGNATURE = b'MSGB'
MAX_STRING_LENGTH = 10_000_000  # 10MB safety limit


@dataclass
class MemoryStats:
    """Memory statistics for a cassette instance"""
    total_pages: int
    total_bytes: int
    usage_estimate: str
    allocation_count: int
    allocated_pointers: List[int]


@dataclass
class CassetteInfo:
    """Information about a loaded cassette"""
    id: str
    name: str
    description: str
    version: str
    author: str
    created: str
    event_count: int


class WasmMemoryManager:
    """Handles memory interactions between Python and WebAssembly"""
    
    def __init__(self, memory: Memory, instance: Instance, store: Store, debug: bool = False):
        self.memory = memory
        self.instance = instance
        self.store = store
        self.debug = debug
        self.allocated_pointers: set[int] = set()
        
    def _log(self, *args):
        """Log debug information if debug mode is enabled"""
        if self.debug:
            print("[WasmMemoryManager]", *args)
            
    def _has_msgb_signature(self, ptr: int) -> bool:
        """Check if a buffer has our MSGB signature"""
        mem_size = self.memory.size(self.store) * 65536  # pages to bytes
        if ptr == 0 or ptr + 4 > mem_size:
            return False
            
        data = self.memory.data_ptr(self.store)
        # Read 4 bytes for signature check
        sig_bytes = bytes([data[ptr + i] for i in range(4)])
        return sig_bytes == MSGB_SIGNATURE
        
    def read_string(self, ptr: int) -> str:
        """Read a string from memory with proper handling for MSGB format"""
        if ptr == 0:
            self._log("Received null pointer")
            return ""
            
        try:
            data = self.memory.data_ptr(self.store)
            mem_size = self.memory.size(self.store) * 65536
            
            # Check for MSGB signature
            if self._has_msgb_signature(ptr):
                self._log("Detected MSGB string format")
                
                # MSGB format: [signature(4 bytes)][length(4 bytes)][data(length bytes)]
                length_bytes = bytes([data[ptr+4+i] for i in range(4)])
                length = struct.unpack('<I', length_bytes)[0]  # Little endian
                
                self._log(f"String length from MSGB format: {length}")
                
                if length > MAX_STRING_LENGTH:
                    raise ValueError(f"String too large ({length} bytes)")
                    
                # Read string data
                string_bytes = bytes([data[ptr+8+i] for i in range(length)])
                result = string_bytes.decode('utf-8')
                
                self._log(f"Read string: {result[:50]}{'...' if len(result) > 50 else ''}")
                return result
                
            # Fallback: try to read as null-terminated string
            self._log("Using fallback method to read string")
            
            # Find null terminator
            end = ptr
            while end < mem_size and data[end] != 0:
                end += 1
                
            if end == ptr:
                self._log("Empty string (null at start)")
                return ""
                
            # Read string bytes
            string_bytes = bytes([data[ptr+i] for i in range(end-ptr)])
            result = string_bytes.decode('utf-8')
            
            self._log(f"Read string using fallback: {result[:50]}{'...' if len(result) > 50 else ''}")
            return result
            
        except Exception as e:
            self._log(f"Error reading string: {e}")
            return ""
            
    def write_string(self, s: str) -> int:
        """Write a string to memory with proper MSGB format"""
        if not s:
            self._log("Empty string provided to write_string, returning 0")
            return 0
            
        self._log(f"Writing string to memory (length {len(s)}): {s[:50]}{'...' if len(s) > 50 else ''}")
        
        # Encode string to UTF-8
        encoded = s.encode('utf-8')
        
        # Try alloc_buffer first (exported by cassette-tools)
        exports = self.instance.exports(self.store)
        if 'alloc_buffer' in exports:
            self._log("Using alloc_buffer function")
            try:
                alloc_func = exports['alloc_buffer']
                ptr = alloc_func(self.store, len(encoded))
                
                if ptr == 0:
                    self._log("alloc_buffer returned null pointer")
                    return 0
                    
                self.allocated_pointers.add(ptr)
                
                # Copy string data to WASM memory
                data = self.memory.data_ptr(self.store)
                for i, byte in enumerate(encoded):
                    data[ptr + i] = byte
                
                self._log(f"String written to memory at pointer {ptr} ({len(encoded)} bytes)")
                return ptr
                
            except Exception as e:
                self._log(f"Error allocating memory with alloc_buffer: {e}")
                return 0
                
        # Try alloc_string as fallback
        elif 'alloc_string' in exports:
            self._log("Using alloc_string function")
            try:
                alloc_func = exports['alloc_string']
                ptr = alloc_func(self.store, len(encoded))
                
                if ptr == 0:
                    self._log("alloc_string returned null pointer")
                    return 0
                    
                self.allocated_pointers.add(ptr)
                
                # Copy string data to WASM memory
                data = self.memory.data_ptr(self.store)
                for i, byte in enumerate(encoded):
                    data[ptr + i] = byte
                
                self._log(f"String written to memory at pointer {ptr} ({len(encoded)} bytes)")
                return ptr
                
            except Exception as e:
                self._log(f"Error allocating memory with alloc_string: {e}")
                return 0
                
        self._log("No allocation function available")
        return 0
        
    def deallocate_string(self, ptr: int) -> None:
        """Deallocate a string from memory with proper handling for MSGB format"""
        if ptr == 0:
            self._log("Ignoring request to deallocate null pointer")
            return
            
        self._log(f"Deallocating string at pointer {ptr}")
        
        # Check if this pointer is tracked
        if ptr not in self.allocated_pointers:
            self._log(f"Warning: Attempting to deallocate untracked pointer {ptr}")
            
        # Try to get allocation size
        allocation_size = 0
        exports = self.instance.exports(self.store)
        if 'get_allocation_size' in exports:
            try:
                get_size_func = exports['get_allocation_size']
                allocation_size = get_size_func(self.store, ptr)
                self._log(f"Got allocation size from get_allocation_size: {allocation_size}")
            except Exception as e:
                self._log(f"Error calling get_allocation_size: {e}")
                
        # If we didn't get a valid size, try to determine it ourselves
        if allocation_size == 0 and self._has_msgb_signature(ptr):
            data = self.memory.data_ptr(self.store)
            length_bytes = bytes([data[ptr+4+i] for i in range(4)])
            length = struct.unpack('<I', length_bytes)[0]
            allocation_size = 8 + length  # MSGB header (8 bytes) + data
            self._log(f"Calculated allocation size from MSGB: {allocation_size}")
            
        # Try to deallocate
        try:
            exports = self.instance.exports(self.store)
            if 'dealloc_string' in exports:
                self._log("Using dealloc_string function")
                dealloc_func = exports['dealloc_string']
                dealloc_func(self.store, ptr, allocation_size)
                self._log(f"dealloc_string call completed successfully with size {allocation_size}")
                self.allocated_pointers.discard(ptr)
                return
                
        except Exception as e:
            self._log(f"Error deallocating memory: {e}")
            
        # Still remove from tracking even if deallocation failed
        self.allocated_pointers.discard(ptr)
        
    def get_allocation_count(self) -> int:
        """Get the number of currently tracked allocations"""
        return len(self.allocated_pointers)
        
    def get_allocated_pointers(self) -> List[int]:
        """Get a list of all currently tracked allocations"""
        return list(self.allocated_pointers)


class Cassette:
    """Represents a loaded cassette instance"""
    
    def __init__(self, wasm_bytes: bytes, name: str, debug: bool = False):
        self.name = name
        self.debug = debug
        self.id = self._generate_id()
        self.returned_events: set[str] = set()
        
        # Initialize wasmtime components
        self.store = Store()
        self.module = Module(self.store.engine, wasm_bytes)
        self.instance = Instance(self.store, self.module, [])
        
        # Get exported memory
        self.memory = self.instance.exports(self.store)["memory"]
        
        # Initialize memory manager
        self.memory_manager = WasmMemoryManager(self.memory, self.instance, self.store, debug)
        
        # Load cassette info
        self.info = self._load_info()
        
    def _generate_id(self) -> str:
        """Generate a unique ID for this cassette instance"""
        timestamp = int(time.time() * 1000)
        return f"{self.name.replace('.wasm', '')}_{timestamp}"
        
    def _load_info(self) -> CassetteInfo:
        """Load cassette information from the describe function"""
        try:
            describe_func = self.instance.exports(self.store)['describe']
            ptr = describe_func(self.store)
            
            if ptr == 0:
                raise ValueError("describe() returned null pointer")
                
            description_json = self.memory_manager.read_string(ptr)
            self.memory_manager.deallocate_string(ptr)
            
            data = json.loads(description_json)
            
            return CassetteInfo(
                id=self.id,
                name=data.get('name', 'Unknown'),
                description=data.get('description', ''),
                version=data.get('version', '0.0.0'),
                author=data.get('author', 'Unknown'),
                created=data.get('created', ''),
                event_count=data.get('event_count', 0)
            )
            
        except Exception as e:
            if self.debug:
                print(f"Error loading cassette info: {e}")
            return CassetteInfo(
                id=self.id,
                name=self.name,
                description='Failed to load description',
                version='0.0.0',
                author='Unknown',
                created='',
                event_count=0
            )
            
    def describe(self) -> str:
        """Get cassette description"""
        try:
            describe_func = self.instance.exports(self.store)['describe']
            ptr = describe_func(self.store)
            
            if ptr == 0:
                return json.dumps({"error": "describe() returned null pointer"})
                
            result = self.memory_manager.read_string(ptr)
            self.memory_manager.deallocate_string(ptr)
            
            return result
            
        except Exception as e:
            return json.dumps({"error": f"Failed to call describe: {e}"})
            
    def req(self, request: str) -> str:
        """Process a REQ message"""
        try:
            # Parse request to check if it's a new REQ
            req_data = json.loads(request)
            if isinstance(req_data, list) and len(req_data) >= 3 and req_data[0] == "REQ":
                # New REQ, reset event tracking
                self.returned_events.clear()
                if self.debug:
                    print(f"[Cassette] New REQ call received, resetting event tracker")
                    
            # Write request to memory
            req_ptr = self.memory_manager.write_string(request)
            if req_ptr == 0:
                return json.dumps(["NOTICE", "Failed to allocate memory for request"])
                
            # Call req function
            req_func = self.instance.exports(self.store)['req']
            result_ptr = req_func(self.store, req_ptr, len(request.encode('utf-8')))
            
            # Read result
            if result_ptr == 0:
                self.memory_manager.deallocate_string(req_ptr)
                return json.dumps(["NOTICE", "req() returned null pointer"])
                
            result = self.memory_manager.read_string(result_ptr)
            
            # Deallocate memory
            self.memory_manager.deallocate_string(req_ptr)
            self.memory_manager.deallocate_string(result_ptr)
            
            # Handle newline-separated messages (like JavaScript loader)
            if '\n' in result:
                messages = result.strip().split('\n')
                if self.debug:
                    print(f"[Cassette] Processing {len(messages)} newline-separated messages")
                
                filtered_messages = []
                for message in messages:
                    try:
                        parsed = json.loads(message)
                        if not isinstance(parsed, list) or len(parsed) < 2:
                            if self.debug:
                                print(f"[Cassette] Invalid message format: {message[:100]}")
                            continue
                        
                        # Validate message type
                        if parsed[0] not in ["NOTICE", "EVENT", "EOSE"]:
                            if self.debug:
                                print(f"[Cassette] Unknown message type: {parsed[0]}")
                            continue
                        
                        # Filter duplicate events
                        if parsed[0] == "EVENT" and len(parsed) >= 3:
                            event_id = parsed[2].get('id', '')
                            if event_id in self.returned_events:
                                if self.debug:
                                    print(f"[Cassette] Filtering duplicate event: {event_id}")
                                continue
                            self.returned_events.add(event_id)
                        
                        filtered_messages.append(message)
                    except Exception as e:
                        if self.debug:
                            print(f"[Cassette] Failed to parse message: {e}")
                        continue
                
                # Return filtered messages as newline-separated string
                return '\n'.join(filtered_messages) if filtered_messages else ""
            
            # Single message - filter as before
            try:
                parsed = json.loads(result)
                if isinstance(parsed, list) and parsed[0] == "EVENT" and len(parsed) >= 3:
                    event_id = parsed[2].get('id', '')
                    if event_id in self.returned_events:
                        if self.debug:
                            print(f"[Cassette] Filtering duplicate event: {event_id}")
                        # Return empty string to indicate no new event
                        return ""
                    self.returned_events.add(event_id)
            except:
                pass
                
            return result
            
        except Exception as e:
            return json.dumps(["NOTICE", f"Failed to process request: {e}"])
            
    def close(self, close_msg: str) -> str:
        """Process a CLOSE message"""
        try:
            # Write close message to memory
            close_ptr = self.memory_manager.write_string(close_msg)
            if close_ptr == 0:
                return json.dumps(["NOTICE", "Failed to allocate memory for close message"])
                
            # Call close function
            close_func = self.instance.exports(self.store)['close']
            result_ptr = close_func(self.store, close_ptr, len(close_msg.encode('utf-8')))
            
            # Read result
            if result_ptr == 0:
                self.memory_manager.deallocate_string(close_ptr)
                return json.dumps(["NOTICE", "close() returned null pointer"])
                
            result = self.memory_manager.read_string(result_ptr)
            
            # Deallocate memory
            self.memory_manager.deallocate_string(close_ptr)
            self.memory_manager.deallocate_string(result_ptr)
            
            return result
            
        except Exception as e:
            return json.dumps(["NOTICE", f"Failed to process close: {e}"])
            
    def info(self) -> str:
        """Get NIP-11 relay information"""
        try:
            # Check if info function exists
            exports = self.instance.exports(self.store)
            if 'info' not in exports:
                # Return minimal info if function not found
                if self.debug:
                    print("[Cassette] No info function found, returning minimal info")
                return json.dumps({"supported_nips": []})
            
            info_func = exports['info']
            ptr = info_func(self.store)
            
            if ptr == 0:
                return json.dumps({"supported_nips": []})
                
            result = self.memory_manager.read_string(ptr)
            self.memory_manager.deallocate_string(ptr)
            
            return result
            
        except Exception as e:
            if self.debug:
                print(f"[Cassette] Error calling info: {e}")
            return json.dumps({"supported_nips": []})
            
    def get_memory_stats(self) -> MemoryStats:
        """Get memory statistics for this cassette"""
        pages = self.memory.size(self.store)
        bytes_size = pages * 65536  # 64KB per page
        allocation_count = self.memory_manager.get_allocation_count()
        
        if allocation_count == 0:
            usage = "No leaks detected"
        else:
            usage = f"Potential memory leak: {allocation_count} allocations"
            
        return MemoryStats(
            total_pages=pages,
            total_bytes=bytes_size,
            usage_estimate=usage,
            allocation_count=allocation_count,
            allocated_pointers=self.memory_manager.get_allocated_pointers()
        )
        
    def dispose(self) -> Dict[str, Any]:
        """Clean up resources"""
        stats = self.get_memory_stats()
        allocations_cleaned = stats.allocation_count
        
        # Clear allocated pointers
        self.memory_manager.allocated_pointers.clear()
        
        if self.debug:
            print(f"[Cassette] Disposing cassette {self.id}")
            if allocations_cleaned > 0:
                print(f"[Cassette] Warning: {allocations_cleaned} allocations were not properly freed")
                
        return {
            "allocationsCleanedUp": allocations_cleaned,
            "status": "disposed"
        }


def load_cassette(wasm_bytes: bytes, name: str = "cassette", debug: bool = False) -> Dict[str, Any]:
    """
    Load a cassette from WASM bytes
    
    Args:
        wasm_bytes: The WASM module bytes
        name: Name for the cassette
        debug: Enable debug logging
        
    Returns:
        Dictionary with 'success' boolean and either 'cassette' or 'error'
    """
    try:
        cassette = Cassette(wasm_bytes, name, debug)
        
        if debug:
            print(f"[load_cassette] Cassette loaded successfully: {cassette.info.name} (v{cassette.info.version})")
            
        return {
            "success": True,
            "cassette": cassette
        }
        
    except Exception as e:
        if debug:
            print(f"[load_cassette] Failed to load cassette: {e}")
            
        return {
            "success": False,
            "error": str(e)
        }


# Example usage
if __name__ == "__main__":
    # Test with a simple cassette
    import sys
    
    if len(sys.argv) < 2:
        print("Usage: python cassette_loader.py <path_to_wasm_file>")
        sys.exit(1)
        
    wasm_path = sys.argv[1]
    
    # Load WASM file
    with open(wasm_path, 'rb') as f:
        wasm_bytes = f.read()
        
    # Load cassette
    result = load_cassette(wasm_bytes, name=wasm_path.split('/')[-1], debug=True)
    
    if result['success']:
        cassette = result['cassette']
        print(f"\n✅ Cassette loaded: {cassette.info.name}")
        print(f"   Version: {cassette.info.version}")
        print(f"   Author: {cassette.info.author}")
        print(f"   Events: {cassette.info.event_count}")
        
        # Test describe
        print("\n📋 Testing describe:")
        print(cassette.describe())
        
        # Test REQ
        print("\n📤 Testing REQ:")
        req_response = cassette.req('["REQ", "sub1", {"kinds": [1], "limit": 3}]')
        print(req_response)
        
        # Get memory stats
        stats = cassette.get_memory_stats()
        print(f"\n💾 Memory stats:")
        print(f"   Allocations: {stats.allocation_count}")
        print(f"   Memory pages: {stats.total_pages}")
        print(f"   Status: {stats.usage_estimate}")
        
        # Clean up
        dispose_result = cassette.dispose()
        print(f"\n🧹 Disposed: {dispose_result}")
        
    else:
        print(f"❌ Failed to load cassette: {result['error']}")