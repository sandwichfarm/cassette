#ifndef CASSETTE_LOADER_HPP
#define CASSETTE_LOADER_HPP

#include <string>
#include <memory>
#include <unordered_set>
#include <mutex>
#include <vector>
#include <wasmtime.hh>
#include <nlohmann/json.hpp>

namespace cassette {

using json = nlohmann::json;

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
    
    wasmtime::Func req_func;
    wasmtime::Func describe_func;
    std::optional<wasmtime::Func> close_func;
    std::optional<wasmtime::Func> dealloc_func;
    std::optional<wasmtime::Func> get_size_func;
    
    bool debug;
    mutable std::mutex mutex;
    
    std::vector<std::string> process_messages(const std::string& result);
    bool is_duplicate_event(const json& parsed);

public:
    Cassette(const std::string& path, bool debug = false);
    
    std::string describe();
    std::string req(const std::string& request);
    std::string close(const std::string& close_msg);
};

} // namespace cassette

#endif // CASSETTE_LOADER_HPP