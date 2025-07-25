#include "cassette_loader.hpp"
#include <iostream>
#include <sstream>
#include <cstring>
#include <stdexcept>

namespace cassette {

// EventTracker implementation
void EventTracker::reset() {
    std::lock_guard<std::mutex> lock(mutex);
    event_ids.clear();
}

bool EventTracker::add_and_check(const std::string& event_id) {
    std::lock_guard<std::mutex> lock(mutex);
    return event_ids.insert(event_id).second;
}

// MemoryManager implementation
MemoryManager::MemoryManager(wasmtime::Store& store, const wasmtime::Instance& instance)
    : store(store) {
    
    auto memory_export = instance.get(store, "memory");
    if (!memory_export) {
        throw std::runtime_error("memory export not found");
    }
    memory = std::get<wasmtime::Memory>(*memory_export);
    
    auto alloc_export = instance.get(store, "alloc_string");
    if (!alloc_export) {
        throw std::runtime_error("alloc_string function not found");
    }
    alloc_func = std::get<wasmtime::Func>(*alloc_export);
}

int32_t MemoryManager::write_string(const std::string& str) {
    std::vector<wasmtime::Val> args = { wasmtime::Val::i32(static_cast<int32_t>(str.length())) };
    auto results = alloc_func.call(store, args);
    
    if (results.empty() || !results[0].i32()) {
        throw std::runtime_error("allocation failed");
    }
    
    int32_t ptr = *results[0].i32();
    if (ptr == 0) {
        throw std::runtime_error("allocation returned null pointer");
    }
    
    auto data = memory.data(store);
    std::memcpy(data.data() + ptr, str.data(), str.length());
    
    return ptr;
}

std::string MemoryManager::read_string(int32_t ptr) {
    if (ptr == 0) {
        throw std::runtime_error("null pointer");
    }
    
    auto data = memory.data(store);
    size_t ptr_size = static_cast<size_t>(ptr);
    
    // Check for MSGB format
    if (ptr_size + 8 <= data.size()) {
        std::string signature(reinterpret_cast<const char*>(data.data() + ptr_size), 4);
        if (signature == "MSGB") {
            // Read length (little endian)
            uint32_t length = 0;
            std::memcpy(&length, data.data() + ptr_size + 4, 4);
            
            if (ptr_size + 8 + length <= data.size()) {
                return std::string(reinterpret_cast<const char*>(data.data() + ptr_size + 8), length);
            }
        }
    }
    
    // Fall back to null-terminated string
    size_t end = ptr_size;
    while (end < data.size() && data[end] != 0) {
        end++;
    }
    
    return std::string(reinterpret_cast<const char*>(data.data() + ptr_size), end - ptr_size);
}

// Cassette implementation
Cassette::Cassette(const std::string& path, bool debug)
    : engine(), store(engine), debug(debug) {
    
    auto module = wasmtime::Module::from_file(engine, path);
    instance = wasmtime::Instance(store, module, {});
    
    memory_manager = std::make_unique<MemoryManager>(store, instance);
    event_tracker = std::make_unique<EventTracker>();
    
    // Get required functions
    auto send_export = instance.get(store, "send");
    if (!send_export || !std::holds_alternative<wasmtime::Func>(*send_export)) {
        throw std::runtime_error("send function not found");
    }
    send_func = std::get<wasmtime::Func>(*send_export);
    
    // Optional functions
    if (auto describe_export = instance.get(store, "describe")) {
        if (std::holds_alternative<wasmtime::Func>(*describe_export)) {
            describe_func = std::get<wasmtime::Func>(*describe_export);
        }
    }
    
    if (auto info_export = instance.get(store, "info")) {
        if (std::holds_alternative<wasmtime::Func>(*info_export)) {
            info_func = std::get<wasmtime::Func>(*info_export);
        }
    }
    
    if (auto dealloc_export = instance.get(store, "dealloc_string")) {
        if (std::holds_alternative<wasmtime::Func>(*dealloc_export)) {
            dealloc_func = std::get<wasmtime::Func>(*dealloc_export);
        }
    }
    
    if (auto get_size_export = instance.get(store, "get_allocation_size")) {
        if (std::holds_alternative<wasmtime::Func>(*get_size_export)) {
            get_size_func = std::get<wasmtime::Func>(*get_size_export);
        }
    }
}

std::string Cassette::describe() {
    std::lock_guard<std::mutex> lock(mutex);
    
    // If describe function exists, use it
    if (describe_func) {
        auto results = describe_func->call(store, {});
        if (results.empty() || !results[0].i32()) {
            throw std::runtime_error("describe function failed");
        }
        
        int32_t ptr = *results[0].i32();
        std::string desc = memory_manager->read_string(ptr);
        
        // Try to deallocate
        if (dealloc_func) {
            std::vector<wasmtime::Val> args = {
                wasmtime::Val::i32(ptr),
                wasmtime::Val::i32(static_cast<int32_t>(desc.length()))
            };
            dealloc_func->call(store, args);
        }
        
        return desc;
    }
    
    // Otherwise, synthesize from info()
    std::string info_str = info();
    try {
        auto info_json = json::parse(info_str);
        std::string description = "Cassette";
        
        if (info_json.contains("name") && info_json["name"].is_string()) {
            description = info_json["name"].get<std::string>();
        }
        
        if (info_json.contains("supported_nips") && info_json["supported_nips"].is_array()) {
            auto nips = info_json["supported_nips"];
            if (!nips.empty()) {
                description += " (supports NIPs: ";
                for (size_t i = 0; i < nips.size(); ++i) {
                    if (i > 0) description += ", ";
                    description += std::to_string(nips[i].get<int>());
                }
                description += ")";
            }
        }
        
        return description;
    } catch (...) {
        return "Cassette module";
    }
}

std::vector<std::string> Cassette::process_messages(const std::string& result) {
    std::vector<std::string> filtered_messages;
    
    if (result.find('\n') != std::string::npos) {
        std::istringstream stream(result);
        std::string message;
        int count = 0;
        
        while (std::getline(stream, message)) {
            if (message.empty()) continue;
            count++;
        }
        
        if (debug) {
            std::cerr << "[Cassette] Processing " << count << " newline-separated messages\n";
        }
        
        stream.clear();
        stream.str(result);
        
        while (std::getline(stream, message)) {
            if (message.empty()) continue;
            
            try {
                auto parsed = json::parse(message);
                
                if (!parsed.is_array() || parsed.size() < 2) {
                    if (debug) {
                        std::cerr << "[Cassette] Invalid message format: " << message.substr(0, 100) << "\n";
                    }
                    continue;
                }
                
                std::string msg_type = parsed[0].get<std::string>();
                if (msg_type != "NOTICE" && msg_type != "EVENT" && msg_type != "EOSE") {
                    if (debug) {
                        std::cerr << "[Cassette] Unknown message type: " << msg_type << "\n";
                    }
                    continue;
                }
                
                // Filter duplicate events
                if (msg_type == "EVENT" && parsed.size() >= 3) {
                    if (is_duplicate_event(parsed)) {
                        continue;
                    }
                }
                
                filtered_messages.push_back(message);
                
            } catch (const std::exception& e) {
                if (debug) {
                    std::cerr << "[Cassette] Failed to parse message: " << e.what() << "\n";
                }
            }
        }
    } else {
        // Single message
        try {
            auto parsed = json::parse(result);
            if (parsed.is_array() && parsed[0] == "EVENT" && parsed.size() >= 3) {
                if (!is_duplicate_event(parsed)) {
                    filtered_messages.push_back(result);
                }
            } else {
                filtered_messages.push_back(result);
            }
        } catch (...) {
            filtered_messages.push_back(result);
        }
    }
    
    return filtered_messages;
}

bool Cassette::is_duplicate_event(const json& parsed) {
    if (parsed[0] == "EVENT" && parsed.size() >= 3 && parsed[2].is_object()) {
        auto event = parsed[2];
        if (event.contains("id") && event["id"].is_string()) {
            std::string event_id = event["id"];
            if (!event_tracker->add_and_check(event_id)) {
                if (debug) {
                    std::cerr << "[Cassette] Filtering duplicate event: " << event_id << "\n";
                }
                return true;
            }
        }
    }
    return false;
}

std::string Cassette::send(const std::string& message) {
    std::lock_guard<std::mutex> lock(mutex);
    
    // Parse message to check for REQ or CLOSE
    try {
        auto msg_data = json::parse(message);
        if (msg_data.is_array() && msg_data.size() >= 2) {
            std::string msg_type = msg_data[0].get<std::string>();
            if (msg_type == "REQ") {
                event_tracker->reset();
                if (debug) {
                    std::cerr << "[Cassette] New REQ, resetting event tracker\n";
                }
            } else if (msg_type == "CLOSE") {
                // Handle CLOSE message if needed
                if (debug) {
                    std::cerr << "[Cassette] Processing CLOSE message\n";
                }
            }
        }
    } catch (...) {}
    
    // Write message to memory
    int32_t msg_ptr = memory_manager->write_string(message);
    
    // Call send function
    std::vector<wasmtime::Val> args = {
        wasmtime::Val::i32(msg_ptr),
        wasmtime::Val::i32(static_cast<int32_t>(message.length()))
    };
    auto results = send_func.call(store, args);
    
    // Deallocate message
    if (dealloc_func) {
        dealloc_func->call(store, args);
    }
    
    if (results.empty() || !results[0].i32()) {
        return R"(["NOTICE", "send() failed"])";
    }
    
    int32_t result_ptr = *results[0].i32();
    if (result_ptr == 0) {
        return R"(["NOTICE", "send() returned null pointer"])";
    }
    
    // Read result
    std::string result_str = memory_manager->read_string(result_ptr);
    
    // Deallocate result
    if (dealloc_func) {
        int32_t size = static_cast<int32_t>(result_str.length());
        if (get_size_func) {
            auto size_results = get_size_func->call(store, {wasmtime::Val::i32(result_ptr)});
            if (!size_results.empty() && size_results[0].i32()) {
                size = *size_results[0].i32();
            }
        }
        dealloc_func->call(store, {wasmtime::Val::i32(result_ptr), wasmtime::Val::i32(size)});
    }
    
    // Process messages
    auto filtered = process_messages(result_str);
    
    if (filtered.empty()) {
        return "";
    } else if (filtered.size() == 1) {
        return filtered[0];
    } else {
        std::ostringstream oss;
        for (size_t i = 0; i < filtered.size(); ++i) {
            if (i > 0) oss << "\n";
            oss << filtered[i];
        }
        return oss.str();
    }
}


std::string Cassette::info() {
    std::lock_guard<std::mutex> lock(mutex);
    
    if (!info_func) {
        // Return minimal info if function not found
        return R"({"supported_nips": []})";
    }
    
    // Call info function
    auto results = info_func->call(store, {});
    
    if (results.empty() || !results[0].i32()) {
        return R"({"supported_nips": []})";
    }
    
    int32_t ptr = *results[0].i32();
    if (ptr == 0) {
        return R"({"supported_nips": []})";
    }
    
    // Read result
    std::string info_str = memory_manager->read_string(ptr);
    
    // Try to deallocate
    if (dealloc_func) {
        dealloc_func->call(store, {
            wasmtime::Val::i32(ptr),
            wasmtime::Val::i32(static_cast<int32_t>(info_str.length()))
        });
    }
    
    return info_str;
}

} // namespace cassette