#include <cassette_loader.hpp>
#include <iostream>
#include <string>

int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::cerr << "Usage: " << argv[0] << " <cassette.wasm>" << std::endl;
        return 1;
    }
    
    try {
        // Load cassette with debug enabled
        cassette::Cassette cassette(argv[1], true);
        
        // Get and display description
        std::string desc = cassette.describe();
        std::cout << "Cassette Description:" << std::endl;
        std::cout << desc << std::endl << std::endl;
        
        // Create a REQ message
        std::string req = R"(["REQ", "example-sub", {"limit": 5}])";
        std::cout << "Sending REQ: " << req << std::endl << std::endl;
        
        // Loop to get all events
        int event_count = 0;
        while (true) {
            std::string result = cassette.send(req);
            
            if (result.empty()) {
                std::cout << "No more events" << std::endl;
                break;
            }
            
            std::cout << "Received: " << result << std::endl;
            
            // Count events
            if (result.find("\"EVENT\"") != std::string::npos) {
                event_count++;
            }
            
            // Check for EOSE
            if (result.find("\"EOSE\"") != std::string::npos) {
                std::cout << "End of stored events" << std::endl;
                break;
            }
        }
        
        std::cout << std::endl << "Total events received: " << event_count << std::endl;
        
        // Test CLOSE using send()
        std::string close_msg = R"(["CLOSE", "example-sub"])";
        std::cout << std::endl << "Sending CLOSE: " << close_msg << std::endl;
        std::string close_result = cassette.send(close_msg);
        std::cout << "CLOSE result: " << close_result << std::endl;
        
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}