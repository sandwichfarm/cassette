#include <iostream>
#include <fstream>
#include <chrono>
#include <vector>
#include <string>
#include <algorithm>
#include <numeric>
#include <sstream>
#include <iomanip>
#include <filesystem>
#include <cstdlib>
#include <ctime>
#include <nlohmann/json.hpp>
#include "cassette_loader.hpp"

using json = nlohmann::json;
using namespace std::chrono;
namespace fs = std::filesystem;

// Generate random hex string
std::string generate_random_hex(int length) {
    static const char* hex_chars = "0123456789abcdef";
    std::string result;
    result.reserve(length);
    for (int i = 0; i < length; ++i) {
        result += hex_chars[rand() % 16];
    }
    return result;
}

// Test filter configurations
struct TestFilter {
    std::string name;
    json filter;
};

std::vector<TestFilter> generate_test_filters() {
    std::vector<TestFilter> filters;
    
    filters.push_back({"empty", json::object()});
    filters.push_back({"limit_1", json{{"limit", 1}}});
    filters.push_back({"limit_10", json{{"limit", 10}}});
    filters.push_back({"limit_100", json{{"limit", 100}}});
    filters.push_back({"limit_1000", json{{"limit", 1000}}});
    filters.push_back({"kinds_1", json{{"kinds", json::array({1})}}});
    filters.push_back({"kinds_multiple", json{{"kinds", json::array({1, 7, 0})}}});
    
    filters.push_back({"author_single", json{{"authors", json::array({generate_random_hex(64)})}}});
    
    std::vector<std::string> authors;
    for (int i = 0; i < 5; ++i) {
        authors.push_back(generate_random_hex(64));
    }
    filters.push_back({"authors_5", json{{"authors", authors}}});
    
    auto now = time(nullptr);
    filters.push_back({"since_recent", json{{"since", now - 3600}}});
    filters.push_back({"until_now", json{{"until", now}}});
    filters.push_back({"time_range", json{{"since", now - 86400}, {"until", now}}});
    
    filters.push_back({"tag_e", json{{"#e", json::array({generate_random_hex(64)})}}});
    filters.push_back({"tag_p", json{{"#p", json::array({generate_random_hex(64)})}}});
    
    filters.push_back({"complex", json{
        {"kinds", json::array({1})},
        {"limit", 50},
        {"since", now - 86400},
        {"authors", json::array({generate_random_hex(64)})}
    }});
    
    return filters;
}

// Benchmark result structure
struct BenchmarkResult {
    std::string cassette_name;
    size_t file_size;
    int event_count;
    std::map<std::string, std::vector<double>> filter_timings;
    std::map<std::string, std::vector<int>> filter_event_counts;
};

// Calculate percentile
double percentile(std::vector<double> values, double p) {
    if (values.empty()) return 0.0;
    std::sort(values.begin(), values.end());
    int index = static_cast<int>(values.size() * p);
    return values[std::min(index, static_cast<int>(values.size() - 1))];
}

// Benchmark a single cassette
BenchmarkResult benchmark_cassette(const std::string& cassette_path, int iterations) {
    std::cout << "\n📼 Benchmarking: " << fs::path(cassette_path).filename() << std::endl;
    std::cout << std::string(60, '=') << std::endl;
    
    BenchmarkResult result;
    result.cassette_name = fs::path(cassette_path).filename();
    
    // Get file size
    result.file_size = fs::file_size(cassette_path);
    
    try {
        // Load cassette
        cassette::Cassette cassette(cassette_path, false);
        
        // Get cassette info
        std::string info_str = cassette.info();
        json info = json::parse(info_str);
        
        result.event_count = info.value("event_count", 0);
        
        std::cout << "ℹ️  Cassette: " << info.value("name", "Unknown") << std::endl;
        std::cout << "   Events: " << result.event_count << std::endl;
        std::cout << "   Size: " << std::fixed << std::setprecision(1) 
                  << result.file_size / 1024.0 << " KB" << std::endl;
        
        // Warm up
        std::cout << "🔥 Warming up..." << std::endl;
        for (int i = 0; i < 10; ++i) {
            json req = json::array({"REQ", "warmup-" + std::to_string(i), json{{"limit", 1}}});
            auto response = cassette.send(req.dump());
        }
        
        // Test filters
        auto test_filters = generate_test_filters();
        
        std::cout << "\n🏃 Running " << iterations << " iterations per filter..." << std::endl;
        
        for (size_t idx = 0; idx < test_filters.size(); ++idx) {
            const auto& test = test_filters[idx];
            std::cout << "\n  Testing filter " << (idx + 1) << "/" << test_filters.size() 
                      << ": " << test.name << std::flush;
            
            std::vector<double> times;
            std::vector<int> event_counts;
            
            for (int i = 0; i < iterations; ++i) {
                if (i % 10 == 0) std::cout << "." << std::flush;
                
                std::string sub_id = "bench-" + test.name + "-" + std::to_string(i);
                json req_message = json::array({"REQ", sub_id, test.filter});
                
                auto start = high_resolution_clock::now();
                auto response = cassette.send(req_message.dump());
                auto end = high_resolution_clock::now();
                
                double elapsed_ms = duration_cast<microseconds>(end - start).count() / 1000.0;
                times.push_back(elapsed_ms);
                
                // Count events returned
                int event_count = 0;
                if (std::holds_alternative<std::vector<std::string>>(response)) {
                    auto& messages = std::get<std::vector<std::string>>(response);
                    for (const auto& msg : messages) {
                        try {
                            json parsed = json::parse(msg);
                            if (parsed[0] == "EVENT") event_count++;
                        } catch (...) {}
                    }
                }
                event_counts.push_back(event_count);
            }
            
            result.filter_timings[test.name] = times;
            result.filter_event_counts[test.name] = event_counts;
            
            double avg_ms = std::accumulate(times.begin(), times.end(), 0.0) / times.size();
            double avg_events = std::accumulate(event_counts.begin(), event_counts.end(), 0.0) / event_counts.size();
            
            std::cout << " ✓ (" << std::fixed << std::setprecision(1) << avg_ms 
                      << "ms avg, " << std::setprecision(0) << avg_events << " events)" << std::endl;
        }
        
    } catch (const std::exception& e) {
        std::cerr << "❌ Error: " << e.what() << std::endl;
    }
    
    return result;
}

// Print comparison table
void print_comparison_table(const std::vector<BenchmarkResult>& results) {
    std::cout << "\n📊 CASSETTE PERFORMANCE COMPARISON" << std::endl;
    std::cout << std::string(100, '=') << std::endl;
    
    std::cout << "\n🔍 REQ QUERY PERFORMANCE (milliseconds)" << std::endl;
    std::cout << std::string(100, '=') << std::endl;
    
    // Collect all filter names
    std::set<std::string> all_filters;
    for (const auto& result : results) {
        for (const auto& [filter_name, _] : result.filter_timings) {
            all_filters.insert(filter_name);
        }
    }
    
    // Print header
    std::cout << std::left << std::setw(20) << "Filter Type";
    for (const auto& result : results) {
        std::string name = result.cassette_name;
        if (name.length() > 12) name = name.substr(0, 12);
        std::cout << std::right << std::setw(12) << name << " ";
    }
    std::cout << std::endl;
    std::cout << std::string(20 + 13 * results.size(), '-') << std::endl;
    
    // Print data
    for (const auto& filter_name : all_filters) {
        std::cout << std::left << std::setw(20) << filter_name;
        for (const auto& result : results) {
            auto it = result.filter_timings.find(filter_name);
            if (it != result.filter_timings.end() && !it->second.empty()) {
                double avg_ms = std::accumulate(it->second.begin(), it->second.end(), 0.0) / it->second.size();
                std::cout << std::right << std::setw(11) << std::fixed << std::setprecision(2) << avg_ms << "  ";
            } else {
                std::cout << std::right << std::setw(11) << "N/A" << "  ";
            }
        }
        std::cout << std::endl;
    }
    
    // Summary stats
    std::cout << "\n📈 SUMMARY STATISTICS" << std::endl;
    std::cout << std::string(100, '=') << std::endl;
    std::cout << std::left << std::setw(30) << "Cassette" 
              << std::right << std::setw(10) << "Size (KB)" 
              << std::setw(10) << "Events"
              << std::setw(10) << "Avg (ms)"
              << std::setw(10) << "P95 (ms)" << std::endl;
    std::cout << std::string(70, '-') << std::endl;
    
    for (const auto& result : results) {
        std::vector<double> all_times;
        for (const auto& [_, times] : result.filter_timings) {
            all_times.insert(all_times.end(), times.begin(), times.end());
        }
        
        if (!all_times.empty()) {
            double avg_time = std::accumulate(all_times.begin(), all_times.end(), 0.0) / all_times.size();
            double p95 = percentile(all_times, 0.95);
            
            std::cout << std::left << std::setw(30) << result.cassette_name
                      << std::right << std::setw(10) << std::fixed << std::setprecision(1) 
                      << result.file_size / 1024.0
                      << std::setw(10) << result.event_count
                      << std::setw(10) << std::setprecision(2) << avg_time
                      << std::setw(10) << p95 << std::endl;
        }
    }
}

int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::cerr << "Usage: " << argv[0] << " <cassette.wasm> [cassette2.wasm ...]" << std::endl;
        return 1;
    }
    
    int iterations = 100;
    
    // Parse command line arguments
    std::vector<std::string> cassette_paths;
    for (int i = 1; i < argc; ++i) {
        std::string arg = argv[i];
        if (arg == "--iterations" || arg == "-i") {
            if (i + 1 < argc) {
                iterations = std::stoi(argv[++i]);
            }
        } else {
            cassette_paths.push_back(arg);
        }
    }
    
    if (cassette_paths.empty()) {
        std::cerr << "Error: No cassette files specified" << std::endl;
        return 1;
    }
    
    // Initialize random seed
    srand(time(nullptr));
    
    std::cout << "🚀 Cassette WASM Benchmark (C++)" << std::endl;
    std::cout << "   Cassettes: " << cassette_paths.size() << std::endl;
    std::cout << "   Iterations: " << iterations << std::endl;
    
    std::vector<BenchmarkResult> results;
    
    for (const auto& path : cassette_paths) {
        if (!fs::exists(path)) {
            std::cerr << "❌ Not found: " << path << std::endl;
            continue;
        }
        
        try {
            results.push_back(benchmark_cassette(path, iterations));
        } catch (const std::exception& e) {
            std::cerr << "❌ Error with " << path << ": " << e.what() << std::endl;
        }
    }
    
    if (!results.empty()) {
        print_comparison_table(results);
        
        // Save results to JSON
        json output;
        output["timestamp"] = time(nullptr);
        output["iterations"] = iterations;
        
        for (const auto& result : results) {
            json cassette_result;
            cassette_result["cassette"] = result.cassette_name;
            cassette_result["file_size"] = result.file_size;
            cassette_result["event_count"] = result.event_count;
            
            json filters;
            for (const auto& [filter_name, times] : result.filter_timings) {
                if (!times.empty()) {
                    filters[filter_name] = {
                        {"count", times.size()},
                        {"avg_ms", std::accumulate(times.begin(), times.end(), 0.0) / times.size()},
                        {"min_ms", *std::min_element(times.begin(), times.end())},
                        {"max_ms", *std::max_element(times.begin(), times.end())},
                        {"p50_ms", percentile(times, 0.5)},
                        {"p95_ms", percentile(times, 0.95)},
                        {"p99_ms", percentile(times, 0.99)}
                    };
                    
                    auto it = result.filter_event_counts.find(filter_name);
                    if (it != result.filter_event_counts.end() && !it->second.empty()) {
                        filters[filter_name]["avg_events"] = 
                            std::accumulate(it->second.begin(), it->second.end(), 0.0) / it->second.size();
                        filters[filter_name]["max_events"] = 
                            *std::max_element(it->second.begin(), it->second.end());
                    }
                }
            }
            cassette_result["filters"] = filters;
            
            output["results"].push_back(cassette_result);
        }
        
        std::string output_filename = "benchmark_cpp_" + std::to_string(time(nullptr)) + ".json";
        std::ofstream output_file(output_filename);
        output_file << output.dump(2);
        output_file.close();
        
        std::cout << "\n💾 Results saved to: " << output_filename << std::endl;
    }
    
    return 0;
}