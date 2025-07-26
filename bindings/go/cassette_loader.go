package cassette

import (
	"encoding/json"
	"fmt"
	"strings"
	"sync"

	"github.com/bytecodealliance/wasmtime-go/v23"
)

// SendResult represents the result of a Send call - either a single response or multiple responses
type SendResult struct {
	IsSingle bool
	Single   string
	Multiple []string
}

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
	requiredFuncs := []string{"send", "info", "dealloc_string"}
	optionalFuncs := []string{"describe"}
	
	for _, name := range requiredFuncs {
		fn := instance.GetFunc(store, name)
		if fn == nil && name != "dealloc_string" {
			return nil, fmt.Errorf("required function %s not found", name)
		}
		if fn != nil {
			exports[name] = fn
		}
	}
	
	for _, name := range optionalFuncs {
		fn := instance.GetFunc(store, name)
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

	// First check if there's a describe function
	descFunc, hasDescribe := c.exports["describe"]
	if hasDescribe {
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

	// Otherwise, synthesize from Info()
	infoFunc, ok := c.exports["info"]
	if !ok {
		return "Cassette with no description", nil
	}

	result, err := infoFunc.Call(c.store)
	if err != nil {
		return "", err
	}

	ptr := result.(int32)
	if ptr == 0 {
		return "Cassette with no description", nil
	}

	infoStr, err := c.memory.ReadString(ptr)
	if err != nil {
		return "", err
	}

	// Try to deallocate
	if deallocFunc, ok := c.exports["dealloc_string"]; ok {
		deallocFunc.Call(c.store, ptr, int32(len(infoStr)))
	}

	// Parse info JSON to create description
	var info map[string]interface{}
	if err := json.Unmarshal([]byte(infoStr), &info); err != nil {
		return "Cassette (invalid info)", nil
	}

	// Build description from info
	var parts []string
	if name, ok := info["name"].(string); ok && name != "" {
		parts = append(parts, name)
	}
	if desc, ok := info["description"].(string); ok && desc != "" {
		parts = append(parts, desc)
	}
	if nips, ok := info["supported_nips"].([]interface{}); ok && len(nips) > 0 {
		nipStrs := make([]string, 0, len(nips))
		for _, nip := range nips {
			if nipNum, ok := nip.(float64); ok {
				nipStrs = append(nipStrs, fmt.Sprintf("NIP-%02d", int(nipNum)))
			}
		}
		if len(nipStrs) > 0 {
			parts = append(parts, fmt.Sprintf("Supports: %s", strings.Join(nipStrs, ", ")))
		}
	}

	if len(parts) > 0 {
		return strings.Join(parts, " - "), nil
	}

	return "Cassette with no description", nil
}

// Send processes any NIP-01 message
// For REQ messages, returns SendResult with Multiple set. For other messages, returns SendResult with Single set.
func (c *Cassette) Send(message string) (*SendResult, error) {
	c.mu.Lock()
	defer c.mu.Unlock()

	// Parse message to determine type
	var isReqMessage bool
	var subscriptionID string

	var msgData []interface{}
	if err := json.Unmarshal([]byte(message), &msgData); err == nil {
		if len(msgData) >= 2 {
			msgType, ok := msgData[0].(string)
			if ok {
				switch msgType {
				case "REQ":
					// New REQ, reset event tracker
					c.eventTracker.Reset()
					if c.debug {
						fmt.Println("[Cassette] New REQ, resetting event tracker")
					}
					isReqMessage = true
					if subID, ok := msgData[1].(string); ok {
						subscriptionID = subID
					}
				case "CLOSE":
					// CLOSE message, reset event tracker for that subscription
					c.eventTracker.Reset()
					if c.debug {
						fmt.Println("[Cassette] CLOSE message, resetting event tracker")
					}
				}
			}
		}
	}

	// If it's a REQ message, collect all events until EOSE
	if isReqMessage {
		results, err := c.collectAllEventsForReq(message, subscriptionID)
		if err != nil {
			return nil, err
		}
		return &SendResult{IsSingle: false, Multiple: results}, nil
	}

	// For non-REQ messages, use single call
	result, err := c.sendSingle(message)
	if err != nil {
		return nil, err
	}
	return &SendResult{IsSingle: true, Single: result}, nil
}

// sendSingle performs a single send call
func (c *Cassette) sendSingle(message string) (string, error) {
	// Write message to memory
	msgPtr, err := c.memory.WriteString(message)
	if err != nil {
		return "", err
	}

	// Call send function
	sendFunc, ok := c.exports["send"]
	if !ok {
		return "", fmt.Errorf("send function not found")
	}

	result, err := sendFunc.Call(c.store, msgPtr, int32(len(message)))
	if err != nil {
		return "", err
	}

	// Deallocate message
	if deallocFunc, ok := c.exports["dealloc_string"]; ok {
		deallocFunc.Call(c.store, msgPtr, int32(len(message)))
	}

	resultPtr := result.(int32)
	if resultPtr == 0 {
		return `["NOTICE", "send() returned null pointer"]`, nil
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

	return c.processResults(resultStr), nil
}

// processResults handles event deduplication and filtering
func (c *Cassette) processResults(resultStr string) string {
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
			if !ok || (msgType != "NOTICE" && msgType != "EVENT" && msgType != "EOSE" && msgType != "OK" && msgType != "AUTH" && msgType != "CLOSED") {
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
			return strings.Join(filteredMessages, "\n")
		}
		return ""
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
						return ""
					}
				}
			}
		}
	}

	return resultStr
}

// collectAllEventsForReq collects all events for a REQ message until EOSE
func (c *Cassette) collectAllEventsForReq(message string, subscriptionID string) ([]string, error) {
	if c.debug {
		fmt.Printf("[Cassette] Collecting all events for REQ subscription: %s\n", subscriptionID)
	}

	var results []string

	// Keep calling until we get EOSE or terminating condition
	for {
		response, err := c.sendSingle(message)
		if err != nil {
			return nil, err
		}

		// Empty response means no more events
		if response == "" {
			if c.debug {
				fmt.Println("[Cassette] Received empty response, stopping")
			}
			break
		}

		// Try to parse the response
		var parsed []interface{}
		if err := json.Unmarshal([]byte(response), &parsed); err != nil {
			if c.debug {
				fmt.Printf("[Cassette] Failed to parse response: %v, stopping\n", err)
			}
			break
		}

		if len(parsed) >= 1 {
			msgType, ok := parsed[0].(string)
			if ok {
				switch msgType {
				case "EOSE":
					if c.debug {
						fmt.Printf("[Cassette] Received EOSE for subscription %s\n", subscriptionID)
					}
					results = append(results, response)
					goto done
				case "CLOSED":
					if c.debug {
						fmt.Printf("[Cassette] Received CLOSED for subscription %s\n", subscriptionID)
					}
					results = append(results, response)
					goto done
				}
			}
		}

		// Add the response to results
		results = append(results, response)
	}

done:
	// Check if we have an EOSE message
	hasEOSE := false
	for _, r := range results {
		var parsed []interface{}
		if err := json.Unmarshal([]byte(r), &parsed); err == nil {
			if len(parsed) >= 1 && parsed[0] == "EOSE" {
				hasEOSE = true
				break
			}
		}
	}

	// If no EOSE, add one
	if !hasEOSE {
		eose, _ := json.Marshal([]interface{}{"EOSE", subscriptionID})
		results = append(results, string(eose))
	}

	return results, nil
}



// Info returns NIP-11 relay information
func (c *Cassette) Info() (string, error) {
	c.mu.Lock()
	defer c.mu.Unlock()

	infoFunc, ok := c.exports["info"]
	if !ok {
		// Return minimal info if function not found
		return `{"supported_nips": []}`, nil
	}

	// Call info function
	result, err := infoFunc.Call(c.store)
	if err != nil {
		return "", err
	}

	ptr := result.(int32)
	if ptr == 0 {
		return `{"supported_nips": []}`, nil
	}

	// Read result
	infoStr, err := c.memory.ReadString(ptr)
	if err != nil {
		return "", err
	}

	// Try to deallocate
	if deallocFunc, ok := c.exports["dealloc_string"]; ok {
		deallocFunc.Call(c.store, ptr, int32(len(infoStr)))
	}

	return infoStr, nil
}