package cassette

import (
	"encoding/json"
	"fmt"
	"strings"
	"sync"

	"github.com/bytecodealliance/wasmtime-go/v23"
)

// EventTracker manages event deduplication
type EventTracker struct {
	mu         sync.Mutex
	eventIDs   map[string]bool
	subscripID string
}

// NewEventTracker creates a new event tracker
func NewEventTracker() *EventTracker {
	return &EventTracker{
		eventIDs: make(map[string]bool),
	}
}

// Reset clears the event tracker
func (et *EventTracker) Reset() {
	et.mu.Lock()
	defer et.mu.Unlock()
	et.eventIDs = make(map[string]bool)
}

// AddAndCheck adds an event ID and returns true if it's new
func (et *EventTracker) AddAndCheck(eventID string) bool {
	et.mu.Lock()
	defer et.mu.Unlock()
	
	if et.eventIDs[eventID] {
		return false
	}
	et.eventIDs[eventID] = true
	return true
}

// MemoryManager handles WASM memory operations
type MemoryManager struct {
	memory    *wasmtime.Memory
	allocFunc *wasmtime.Func
	store     *wasmtime.Store
}

// NewMemoryManager creates a new memory manager
func NewMemoryManager(store *wasmtime.Store, instance *wasmtime.Instance) (*MemoryManager, error) {
	memory := instance.GetExport(store, "memory").Memory()
	if memory == nil {
		return nil, fmt.Errorf("memory export not found")
	}

	allocFunc := instance.GetFunc(store, "alloc_string")
	if allocFunc == nil {
		return nil, fmt.Errorf("alloc_string function not found")
	}

	return &MemoryManager{
		memory:    memory,
		allocFunc: allocFunc,
		store:     store,
	}, nil
}

// WriteString writes a string to WASM memory
func (mm *MemoryManager) WriteString(s string) (int32, error) {
	data := []byte(s)
	ptr, err := mm.allocFunc.Call(mm.store, int32(len(data)))
	if err != nil {
		return 0, err
	}

	ptrInt := ptr.(int32)
	if ptrInt == 0 {
		return 0, fmt.Errorf("allocation failed")
	}

	memData := mm.memory.UnsafeData(mm.store)
	copy(memData[ptrInt:], data)
	
	return ptrInt, nil
}

// ReadString reads a string from WASM memory (handles MSGB format)
func (mm *MemoryManager) ReadString(ptr int32) (string, error) {
	if ptr == 0 {
		return "", fmt.Errorf("null pointer")
	}

	memData := mm.memory.UnsafeData(mm.store)
	
	// Check for MSGB signature
	if ptr+8 <= int32(len(memData)) {
		signature := string(memData[ptr:ptr+4])
		if signature == "MSGB" {
			// Read length from bytes 4-7 (little endian)
			length := int32(memData[ptr+4]) |
				int32(memData[ptr+5])<<8 |
				int32(memData[ptr+6])<<16 |
				int32(memData[ptr+7])<<24
			
			if ptr+8+length <= int32(len(memData)) {
				return string(memData[ptr+8:ptr+8+length]), nil
			}
		}
	}
	
	// Fall back to null-terminated string
	end := ptr
	for end < int32(len(memData)) && memData[end] != 0 {
		end++
	}
	
	return string(memData[ptr:end]), nil
}

// Cassette represents a loaded cassette
type Cassette struct {
	engine       *wasmtime.Engine
	store        *wasmtime.Store
	instance     *wasmtime.Instance
	memory       *MemoryManager
	eventTracker *EventTracker
	exports      map[string]*wasmtime.Func
	debug        bool
	mu           sync.Mutex
}

// LoadCassette loads a cassette from a WASM file
func LoadCassette(path string, debug bool) (*Cassette, error) {
	engine := wasmtime.NewEngine()
	module, err := wasmtime.NewModuleFromFile(engine, path)
	if err != nil {
		return nil, fmt.Errorf("failed to load module: %w", err)
	}

	store := wasmtime.NewStore(engine)
	instance, err := wasmtime.NewInstance(store, module, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to instantiate module: %w", err)
	}

	memMgr, err := NewMemoryManager(store, instance)
	if err != nil {
		return nil, fmt.Errorf("failed to create memory manager: %w", err)
	}

	// Get exported functions
	exports := make(map[string]*wasmtime.Func)
	requiredFuncs := []string{"req", "describe", "close", "dealloc_string"}
	
	for _, name := range requiredFuncs {
		fn := instance.GetFunc(store, name)
		if fn == nil && name != "close" && name != "dealloc_string" {
			return nil, fmt.Errorf("required function %s not found", name)
		}
		if fn != nil {
			exports[name] = fn
		}
	}

	return &Cassette{
		engine:       engine,
		store:        store,
		instance:     instance,
		memory:       memMgr,
		eventTracker: NewEventTracker(),
		exports:      exports,
		debug:        debug,
	}, nil
}

// Describe returns the cassette description
func (c *Cassette) Describe() (string, error) {
	c.mu.Lock()
	defer c.mu.Unlock()

	descFunc, ok := c.exports["describe"]
	if !ok {
		return "", fmt.Errorf("describe function not found")
	}

	result, err := descFunc.Call(c.store)
	if err != nil {
		return "", err
	}

	ptr := result.(int32)
	desc, err := c.memory.ReadString(ptr)
	if err != nil {
		return "", err
	}

	// Try to deallocate
	if deallocFunc, ok := c.exports["dealloc_string"]; ok {
		deallocFunc.Call(c.store, ptr, int32(len(desc)))
	}

	return desc, nil
}

// Req processes a REQ message
func (c *Cassette) Req(request string) (string, error) {
	c.mu.Lock()
	defer c.mu.Unlock()

	// Parse request to check for new REQ
	var reqData []interface{}
	if err := json.Unmarshal([]byte(request), &reqData); err == nil {
		if len(reqData) >= 2 && reqData[0] == "REQ" {
			// New REQ, reset event tracker
			c.eventTracker.Reset()
			if c.debug {
				fmt.Println("[Cassette] New REQ, resetting event tracker")
			}
		}
	}

	// Write request to memory
	reqPtr, err := c.memory.WriteString(request)
	if err != nil {
		return "", err
	}

	// Call req function
	reqFunc, ok := c.exports["req"]
	if !ok {
		return "", fmt.Errorf("req function not found")
	}

	result, err := reqFunc.Call(c.store, reqPtr, int32(len(request)))
	if err != nil {
		return "", err
	}

	// Deallocate request
	if deallocFunc, ok := c.exports["dealloc_string"]; ok {
		deallocFunc.Call(c.store, reqPtr, int32(len(request)))
	}

	resultPtr := result.(int32)
	if resultPtr == 0 {
		return `["NOTICE", "req() returned null pointer"]`, nil
	}

	// Read result
	resultStr, err := c.memory.ReadString(resultPtr)
	if err != nil {
		return "", err
	}

	// Deallocate result
	if deallocFunc, ok := c.exports["dealloc_string"]; ok {
		// Try to get size from get_allocation_size if available
		if getSizeFunc := c.instance.GetFunc(c.store, "get_allocation_size"); getSizeFunc != nil {
			if size, err := getSizeFunc.Call(c.store, resultPtr); err == nil {
				deallocFunc.Call(c.store, resultPtr, size.(int32))
			}
		} else {
			deallocFunc.Call(c.store, resultPtr, int32(len(resultStr)))
		}
	}

	// Handle newline-separated messages
	if strings.Contains(resultStr, "\n") {
		messages := strings.Split(strings.TrimSpace(resultStr), "\n")
		if c.debug {
			fmt.Printf("[Cassette] Processing %d newline-separated messages\n", len(messages))
		}

		var filteredMessages []string
		for _, message := range messages {
			var parsed []interface{}
			if err := json.Unmarshal([]byte(message), &parsed); err != nil {
				if c.debug {
					fmt.Printf("[Cassette] Failed to parse message: %v\n", err)
				}
				continue
			}

			if len(parsed) < 2 {
				if c.debug {
					fmt.Printf("[Cassette] Invalid message format: %s\n", message)
				}
				continue
			}

			msgType, ok := parsed[0].(string)
			if !ok || (msgType != "NOTICE" && msgType != "EVENT" && msgType != "EOSE") {
				if c.debug {
					fmt.Printf("[Cassette] Unknown message type: %v\n", parsed[0])
				}
				continue
			}

			// Filter duplicate events
			if msgType == "EVENT" && len(parsed) >= 3 {
				if eventMap, ok := parsed[2].(map[string]interface{}); ok {
					if eventID, ok := eventMap["id"].(string); ok {
						if !c.eventTracker.AddAndCheck(eventID) {
							if c.debug {
								fmt.Printf("[Cassette] Filtering duplicate event: %s\n", eventID)
							}
							continue
						}
					}
				}
			}

			filteredMessages = append(filteredMessages, message)
		}

		if len(filteredMessages) > 0 {
			return strings.Join(filteredMessages, "\n"), nil
		}
		return "", nil
	}

	// Single message - check for duplicate
	var parsed []interface{}
	if err := json.Unmarshal([]byte(resultStr), &parsed); err == nil {
		if len(parsed) >= 3 && parsed[0] == "EVENT" {
			if eventMap, ok := parsed[2].(map[string]interface{}); ok {
				if eventID, ok := eventMap["id"].(string); ok {
					if !c.eventTracker.AddAndCheck(eventID) {
						if c.debug {
							fmt.Printf("[Cassette] Filtering duplicate event: %s\n", eventID)
						}
						return "", nil
					}
				}
			}
		}
	}

	return resultStr, nil
}

// Close processes a CLOSE message
func (c *Cassette) Close(closeMsg string) (string, error) {
	c.mu.Lock()
	defer c.mu.Unlock()

	closeFunc, ok := c.exports["close"]
	if !ok {
		return `["NOTICE", "close function not implemented"]`, nil
	}

	// Write close message to memory
	closePtr, err := c.memory.WriteString(closeMsg)
	if err != nil {
		return "", err
	}

	// Call close function
	result, err := closeFunc.Call(c.store, closePtr, int32(len(closeMsg)))
	if err != nil {
		return "", err
	}

	// Deallocate close message
	if deallocFunc, ok := c.exports["dealloc_string"]; ok {
		deallocFunc.Call(c.store, closePtr, int32(len(closeMsg)))
	}

	resultPtr := result.(int32)
	if resultPtr == 0 {
		return `["NOTICE", "close() returned null pointer"]`, nil
	}

	// Read result
	resultStr, err := c.memory.ReadString(resultPtr)
	if err != nil {
		return "", err
	}

	// Deallocate result
	if deallocFunc, ok := c.exports["dealloc_string"]; ok {
		deallocFunc.Call(c.store, resultPtr, int32(len(resultStr)))
	}

	return resultStr, nil
}