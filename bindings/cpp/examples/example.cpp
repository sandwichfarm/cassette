#include <cassette_loader.hpp>
#include <iostream>
#include <string>
#include <variant>

int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::cerr << "Usage: " << argv[0] << " <cassette.wasm>" << std::endl;
        return 1;
    }
    
    try {
        // Load cassette with debug enabled
        cassette::Cassette cassette(argv[1], true);
        
        // Get and display info
        std::string desc = cassette.info();
        std::cout << "Cassette Info:" << std::endl;
        std::cout << desc << std::endl << std::endl;
        
        // Create a REQ message
        std::string req = R"(["REQ", "example-sub", {"limit": 5}])";
        std::cout << "Sending REQ: " << req << std::endl << std::endl;
        
        // Send REQ - automatically collects all events until EOSE
        auto result = cassette.send(req);
        
        int event_count = 0;
        if (std::holds_alternative<std::vector<std::string>>(result)) {
            // REQ messages return a vector of events
            const auto& events = std::get<std::vector<std::string>>(result);
            for (const auto& event : events) {
                std::cout << "Received: " << event << std::endl;
                if (event.find("\"EVENT\"") != std::string::npos) {
                    event_count++;
                }
            }
        } else {
            // Other messages return a single string
            const auto& response = std::get<std::string>(result);
            std::cout << "Received: " << response << std::endl;
        }
        
        std::cout << std::endl << "Total events received: " << event_count << std::endl;
        
        // Test CLOSE using send()
        std::string close_msg = R"(["CLOSE", "example-sub"])";
        std::cout << std::endl << "Sending CLOSE: " << close_msg << std::endl;
        auto close_result = cassette.send(close_msg);
        if (std::holds_alternative<std::string>(close_result)) {
            std::cout << "CLOSE result: " << std::get<std::string>(close_result) << std::endl;
        }
        
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}