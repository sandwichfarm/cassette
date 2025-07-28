#ifndef CASSETTE_LOADER_HPP
#define CASSETTE_LOADER_HPP

#include <string>
#include <memory>
#include <unordered_set>
#include <mutex>
#include <vector>
#include <variant>
#include <wasmtime.hh>
#include <nlohmann/json.hpp>

namespace cassette {

using json = nlohmann::json;

// Result type for send method - either single response or multiple responses
using SendResult = std::variant<std::string, std::vector<std::string>>;

class EventTracker {
private:
    std::unordered_set<std::string> event_ids;
    mutable std::mutex mutex;

public:
    void reset();
    bool add_and_check(const std::string& event_id);
};

class MemoryManager {
private:
    wasmtime::Memory memory;
    wasmtime::Func alloc_func;
    wasmtime::Store& store;

public:
    MemoryManager(wasmtime::Store& store, const wasmtime::Instance& instance);
    
    int32_t write_string(const std::string& str);
    std::string read_string(int32_t ptr);
    void deallocate(int32_t ptr, int32_t size);
};

class Cassette {
private:
    wasmtime::Engine engine;
    wasmtime::Store store;
    wasmtime::Instance instance;
    std::unique_ptr<MemoryManager> memory_manager;
    std::unique_ptr<EventTracker> event_tracker;
    
    wasmtime::Func scrub_func;
    std::optional<wasmtime::Func> describe_func;
    std::optional<wasmtime::Func> info_func;
    std::optional<wasmtime::Func> dealloc_func;
    std::optional<wasmtime::Func> get_size_func;
    
    bool debug;
    mutable std::mutex mutex;
    
    std::vector<std::string> process_messages(const std::string& result);
    bool is_duplicate_event(const json& parsed);
    std::string send_single(const std::string& message);
    std::vector<std::string> collect_all_events_for_req(const std::string& message, const std::string& subscription_id);

public:
    Cassette(const std::string& path, bool debug = false);
    
    std::string describe();
    SendResult scrub(const std::string& message);
    SendResult send(const std::string& message);  // Deprecated: Use scrub() instead
    std::string info();
};

} // namespace cassette

#endif // CASSETTE_LOADER_HPP