use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use prettytable::{row, Cell, Row, Table};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use wasmtime::*;

const MSGB_SIGNATURE: &[u8] = b"MSGB";
const PAGE_SIZE: usize = 65536;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Cassette WASM files to benchmark
    cassettes: Vec<PathBuf>,

    /// Number of iterations per test (overrides config)
    #[arg(short, long)]
    iterations: Option<usize>,

    /// Save results to JSON file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Enable debug output
    #[arg(short, long)]
    debug: bool,
}

#[derive(Debug, Deserialize)]
struct BenchmarkConfig {
    iterations: HashMap<String, usize>,
    default_iterations: Option<usize>,
    size_patterns: HashMap<String, Vec<String>>,
    language_overrides: Option<HashMap<String, LanguageConfig>>,
}

#[derive(Debug, Deserialize)]
struct LanguageConfig {
    iterations: Option<HashMap<String, usize>>,
}

impl BenchmarkConfig {
    fn load() -> Result<Self> {
        let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("config.yaml");
        
        let contents = fs::read_to_string(&config_path)
            .context("Failed to read config.yaml")?;
        
        serde_yaml::from_str(&contents)
            .context("Failed to parse config.yaml")
    }
    
    fn get_iterations_for_cassette(&self, cassette_path: &Path) -> usize {
        let filename = cassette_path.file_name()
            .unwrap()
            .to_string_lossy()
            .to_lowercase();
        
        // Check language-specific overrides for Rust
        if let Some(overrides) = &self.language_overrides {
            if let Some(rust_config) = overrides.get("rust") {
                if let Some(iterations) = &rust_config.iterations {
                    // Check size patterns
                    for (size, patterns) in &self.size_patterns {
                        for pattern in patterns {
                            let pattern_check = pattern.replace("*", "");
                            if filename.contains(&pattern_check) {
                                if let Some(&iters) = iterations.get(size) {
                                    return iters;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Check global iterations based on size patterns
        for (size, patterns) in &self.size_patterns {
            for pattern in patterns {
                let pattern_check = pattern.replace("*", "");
                if filename.contains(&pattern_check) {
                    if let Some(&iters) = self.iterations.get(size) {
                        return iters;
                    }
                }
            }
        }
        
        // Default fallback
        self.default_iterations.unwrap_or(100)
    }
}

#[derive(Debug, Serialize)]
struct BenchmarkResult {
    cassette: String,
    file_size: usize,
    event_count: u32,
    info: CassetteInfo,
    filters: HashMap<String, FilterResult>,
    memory: MemoryStats,
}

#[derive(Debug, Serialize)]
struct FilterResult {
    count: usize,
    avg_ms: f64,
    min_ms: f64,
    max_ms: f64,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    avg_events: f64,
    max_events: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct CassetteInfo {
    name: String,
    description: String,
    version: String,
    author: String,
    created: String,
    event_count: u32,
}

#[derive(Debug, Serialize)]
struct MemoryStats {
    total_pages: u32,
    total_bytes: usize,
    allocation_count: usize,
}

struct CassetteBenchmark {
    _engine: Engine,
    _module: Module,
    store: Store<()>,
    instance: Instance,
    memory: Memory,
    allocated_pointers: Vec<u32>,
}

impl CassetteBenchmark {
    fn new(wasm_bytes: &[u8]) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::new(&engine, wasm_bytes)?;
        let mut store = Store::new(&engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        let memory = instance.get_memory(&mut store, "memory")
            .context("Failed to get memory export")?;

        Ok(Self {
            _engine: engine,
            _module: module,
            store,
            instance,
            memory,
            allocated_pointers: Vec::new(),
        })
    }

    fn write_string(&mut self, s: &str) -> Result<u32> {
        let bytes = s.as_bytes();
        
        // Try to allocate using alloc_buffer
        if let Ok(alloc_func) = self.instance.get_typed_func::<u32, u32>(&mut self.store, "alloc_buffer") {
            let ptr = alloc_func.call(&mut self.store, bytes.len() as u32)?;
            if ptr == 0 {
                anyhow::bail!("Failed to allocate memory");
            }
            
            // Write bytes to memory
            let data = self.memory.data_mut(&mut self.store);
            data[ptr as usize..ptr as usize + bytes.len()].copy_from_slice(bytes);
            
            self.allocated_pointers.push(ptr);
            return Ok(ptr);
        }
        
        anyhow::bail!("No allocation function found")
    }

    fn read_string(&mut self, ptr: u32) -> Result<String> {
        if ptr == 0 {
            return Ok(String::new());
        }

        let data = self.memory.data(&self.store);
        let mem_size = self.memory.size(&self.store) as usize * PAGE_SIZE;

        // Check for MSGB signature
        if ptr as usize + 8 <= mem_size && &data[ptr as usize..ptr as usize + 4] == MSGB_SIGNATURE {
            // Read length (little-endian)
            let length = u32::from_le_bytes([
                data[ptr as usize + 4],
                data[ptr as usize + 5],
                data[ptr as usize + 6],
                data[ptr as usize + 7],
            ]) as usize;

            if ptr as usize + 8 + length > mem_size {
                anyhow::bail!("String extends beyond memory bounds");
            }

            let string_bytes = &data[ptr as usize + 8..ptr as usize + 8 + length];
            return Ok(String::from_utf8(string_bytes.to_vec())?);
        }

        // Fallback: null-terminated string
        let mut end = ptr as usize;
        while end < mem_size && data[end] != 0 {
            end += 1;
        }

        let string_bytes = &data[ptr as usize..end];
        Ok(String::from_utf8(string_bytes.to_vec())?)
    }

    fn deallocate_string(&mut self, ptr: u32) -> Result<()> {
        if ptr == 0 {
            return Ok(());
        }

        // Calculate size
        let data = self.memory.data(&self.store);
        let size = if ptr as usize + 8 <= data.len() && &data[ptr as usize..ptr as usize + 4] == MSGB_SIGNATURE {
            let length = u32::from_le_bytes([
                data[ptr as usize + 4],
                data[ptr as usize + 5],
                data[ptr as usize + 6],
                data[ptr as usize + 7],
            ]);
            8 + length
        } else {
            0
        };

        if let Ok(dealloc_func) = self.instance.get_typed_func::<(u32, u32), ()>(&mut self.store, "dealloc_string") {
            dealloc_func.call(&mut self.store, (ptr, size))?;
        }

        self.allocated_pointers.retain(|&p| p != ptr);
        Ok(())
    }

    fn scrub(&mut self, message: &str) -> Result<Vec<String>> {
        let msg_ptr = self.write_string(message)?;
        let msg_len = message.len() as u32;

        let send_func = self.instance.get_typed_func::<(u32, u32), u32>(&mut self.store, "send")
            .context("Failed to get send function")?;

        let mut results = Vec::new();
        
        // Parse message to check if it's a REQ
        let is_req = if let Ok(parsed) = serde_json::from_str::<Value>(message) {
            parsed.as_array().map(|arr| arr.get(0).and_then(|v| v.as_str()) == Some("REQ")).unwrap_or(false)
        } else {
            false
        };

        if is_req {
            // For REQ messages, loop until EOSE
            loop {
                let result_ptr = send_func.call(&mut self.store, (msg_ptr, msg_len))?;
                
                if result_ptr == 0 {
                    break;
                }

                let result = self.read_string(result_ptr)?;
                self.deallocate_string(result_ptr)?;

                if result.is_empty() {
                    break;
                }

                // Check for EOSE
                if let Ok(parsed) = serde_json::from_str::<Value>(&result) {
                    if let Some(arr) = parsed.as_array() {
                        if arr.get(0).and_then(|v| v.as_str()) == Some("EOSE") {
                            results.push(result);
                            break;
                        }
                    }
                }

                results.push(result);
            }
        } else {
            // Single response for non-REQ messages
            let result_ptr = send_func.call(&mut self.store, (msg_ptr, msg_len))?;
            if result_ptr != 0 {
                let result = self.read_string(result_ptr)?;
                self.deallocate_string(result_ptr)?;
                results.push(result);
            }
        }

        self.deallocate_string(msg_ptr)?;
        Ok(results)
    }

    fn get_info(&mut self) -> Result<CassetteInfo> {
        // Try info function first (standard cassettes)
        if let Ok(info_func) = self.instance.get_typed_func::<(), u32>(&mut self.store, "info") {
            let ptr = info_func.call(&mut self.store, ())?;
            if ptr == 0 {
                anyhow::bail!("info() returned null");
            }

            let json_str = self.read_string(ptr)?;
            self.deallocate_string(ptr)?;

            // Parse the NIP-11 info and extract what we need
            let info_json: Value = serde_json::from_str(&json_str)?;
            
            return Ok(CassetteInfo {
                name: info_json.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                description: info_json.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                version: info_json.get("version").and_then(|v| v.as_str()).unwrap_or("1.0.0").to_string(),
                author: info_json.get("pubkey").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string(),
                created: "".to_string(),
                event_count: 0, // Will be filled from actual event count
            });
        }

        // Fallback to describe function
        if let Ok(describe_func) = self.instance.get_typed_func::<(), u32>(&mut self.store, "describe") {
            let ptr = describe_func.call(&mut self.store, ())?;
            if ptr == 0 {
                anyhow::bail!("describe() returned null");
            }

            let json_str = self.read_string(ptr)?;
            self.deallocate_string(ptr)?;

            return Ok(serde_json::from_str(&json_str)?);
        }

        // If no info or describe function, return a default
        Ok(CassetteInfo {
            name: "Benchmark Cassette".to_string(),
            description: "Cassette for benchmarking".to_string(),
            version: "1.0.0".to_string(),
            author: "Unknown".to_string(),
            created: "".to_string(),
            event_count: 0, // Will be filled from actual event count
        })
    }

    fn get_event_count(&mut self) -> Result<u32> {
        // Use COUNT query (NIP-45) to get event count
        let count_req = json!(["COUNT", "count-sub", {}]).to_string();
        let responses = self.send(&count_req)?;
        
        for response in responses {
            if let Ok(parsed) = serde_json::from_str::<Value>(&response) {
                if let Some(arr) = parsed.as_array() {
                    if arr.get(0).and_then(|v| v.as_str()) == Some("COUNT") && arr.len() >= 3 {
                        if let Some(count_obj) = arr.get(2) {
                            if let Some(count) = count_obj.get("count").and_then(|v| v.as_u64()) {
                                return Ok(count as u32);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(0)
    }

    fn get_memory_stats(&self) -> MemoryStats {
        let pages = self.memory.size(&self.store) as u32;
        MemoryStats {
            total_pages: pages,
            total_bytes: pages as usize * PAGE_SIZE,
            allocation_count: self.allocated_pointers.len(),
        }
    }
}

fn generate_test_filters() -> Vec<(String, Value)> {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    vec![
        ("empty".to_string(), json!({})),
        ("limit_1".to_string(), json!({"limit": 1})),
        ("limit_10".to_string(), json!({"limit": 10})),
        ("limit_100".to_string(), json!({"limit": 100})),
        ("limit_1000".to_string(), json!({"limit": 1000})),
        ("kinds_1".to_string(), json!({"kinds": [1]})),
        ("kinds_multiple".to_string(), json!({"kinds": [1, 7, 0]})),
        ("author_single".to_string(), json!({"authors": [format!("{:064x}", rng.gen::<u64>())]})),
        ("authors_5".to_string(), json!({"authors": (0..5).map(|_| format!("{:064x}", rng.gen::<u64>())).collect::<Vec<_>>()})),
        ("since_recent".to_string(), json!({"since": Utc::now().timestamp() - 3600})),
        ("until_now".to_string(), json!({"until": Utc::now().timestamp()})),
        ("time_range".to_string(), json!({"since": Utc::now().timestamp() - 86400, "until": Utc::now().timestamp()})),
        ("tag_e".to_string(), json!({"#e": [format!("{:064x}", rng.gen::<u64>())]})),
        ("tag_p".to_string(), json!({"#p": [format!("{:064x}", rng.gen::<u64>())]})),
        ("complex".to_string(), json!({
            "kinds": [1],
            "limit": 50,
            "since": Utc::now().timestamp() - 86400,
            "authors": [format!("{:064x}", rng.gen::<u64>())]
        })),
    ]
}

fn benchmark_cassette(path: &Path, iterations: usize, _debug: bool) -> Result<BenchmarkResult> {
    println!("\n📼 Benchmarking: {}", path.file_name().unwrap().to_string_lossy());
    println!("{}", "=".repeat(60));

    let wasm_bytes = fs::read(path)?;
    let mut cassette = CassetteBenchmark::new(&wasm_bytes)?;

    let mut info = cassette.get_info()?;
    let event_count = cassette.get_event_count()?;
    info.event_count = event_count;
    
    println!("ℹ️  Cassette: {}", path.file_name().unwrap().to_string_lossy());
    println!("   Events: {}", event_count);
    println!("   Size: {:.1} KB", wasm_bytes.len() as f64 / 1024.0);
    println!("   Iterations: {}", iterations);

    // Warmup
    println!("🔥 Warming up...");
    let pb = ProgressBar::new(10);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7}")?);
    
    for _ in 0..10 {
        let _ = cassette.send(&json!(["REQ", "warmup", {"limit": 1}]).to_string());
        pb.inc(1);
    }
    pb.finish_and_clear();

    // Test filters
    let test_filters = generate_test_filters();
    let mut filter_results = HashMap::new();

    println!("\n🏃 Running {} iterations per filter...", iterations);
    let pb = ProgressBar::new((test_filters.len() * iterations) as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")?);

    for (filter_name, filter_obj) in &test_filters {
        pb.set_message(format!("Testing {}", filter_name));
        
        let mut times = Vec::new();
        let mut event_counts = Vec::new();

        for i in 0..iterations {
            let sub_id = format!("bench-{}-{}", filter_name, i);
            let req = json!(["REQ", sub_id, filter_obj]).to_string();

            let start = Instant::now();
            let responses = cassette.send(&req)?;
            let elapsed = start.elapsed();

            times.push(elapsed.as_secs_f64() * 1000.0);

            // Count events
            let event_count = responses.iter()
                .filter(|r| {
                    if let Ok(parsed) = serde_json::from_str::<Value>(r) {
                        parsed.as_array().and_then(|arr| arr.get(0)).and_then(|v| v.as_str()) == Some("EVENT")
                    } else {
                        false
                    }
                })
                .count() as u32;
            event_counts.push(event_count);

            pb.inc(1);
        }

        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        filter_results.insert(filter_name.clone(), FilterResult {
            count: times.len(),
            avg_ms: times.iter().sum::<f64>() / times.len() as f64,
            min_ms: *times.first().unwrap(),
            max_ms: *times.last().unwrap(),
            p50_ms: times[times.len() / 2],
            p95_ms: times[(times.len() as f64 * 0.95) as usize],
            p99_ms: times[(times.len() as f64 * 0.99) as usize],
            avg_events: event_counts.iter().sum::<u32>() as f64 / event_counts.len() as f64,
            max_events: *event_counts.iter().max().unwrap_or(&0),
        });
    }
    pb.finish_and_clear();

    let memory_stats = cassette.get_memory_stats();

    Ok(BenchmarkResult {
        cassette: path.file_name().unwrap().to_string_lossy().to_string(),
        file_size: wasm_bytes.len(),
        event_count: event_count,
        info,
        filters: filter_results,
        memory: memory_stats,
    })
}

fn print_comparison_table(results: &[BenchmarkResult]) {
    println!("\n📊 CASSETTE PERFORMANCE COMPARISON");
    println!("{}", "=".repeat(100));

    // REQ Query Performance
    println!("\n🔍 REQ QUERY PERFORMANCE (milliseconds)");
    let mut table = Table::new();
    
    // Header
    let mut header = vec![Cell::new("Filter Type")];
    for result in results {
        header.push(Cell::new(&result.cassette[..result.cassette.len().min(12)]));
    }
    table.add_row(Row::new(header));

    // Get all filter names
    let mut filter_names: Vec<String> = results[0].filters.keys().cloned().collect();
    filter_names.sort();

    // Data rows
    for filter_name in &filter_names {
        let mut row = vec![Cell::new(filter_name)];
        for result in results {
            if let Some(filter_result) = result.filters.get(filter_name) {
                row.push(Cell::new(&format!("{:.2}", filter_result.avg_ms)));
            } else {
                row.push(Cell::new("N/A"));
            }
        }
        table.add_row(Row::new(row));
    }
    
    table.printstd();

    // Summary stats
    println!("\n📈 SUMMARY STATISTICS");
    let mut summary_table = Table::new();
    summary_table.add_row(row!["Cassette", "Size (KB)", "Events", "Avg (ms)", "P95 (ms)"]);
    
    for result in results {
        let all_avg: Vec<f64> = result.filters.values().map(|f| f.avg_ms).collect();
        let all_p95: Vec<f64> = result.filters.values().map(|f| f.p95_ms).collect();
        let avg_time = all_avg.iter().sum::<f64>() / all_avg.len() as f64;
        let avg_p95 = all_p95.iter().sum::<f64>() / all_p95.len() as f64;
        
        summary_table.add_row(row![
            result.cassette,
            format!("{:.1}", result.file_size as f64 / 1024.0),
            result.event_count,
            format!("{:.2}", avg_time),
            format!("{:.2}", avg_p95)
        ]);
    }
    
    summary_table.printstd();

    // Event retrieval stats
    println!("\n📦 EVENT RETRIEVAL STATISTICS");
    let mut event_table = Table::new();
    
    let mut header = vec![Cell::new("Filter Type")];
    for result in results {
        header.push(Cell::new(&format!("{} (avg)", &result.cassette[..result.cassette.len().min(12)])));
    }
    event_table.add_row(Row::new(header));

    for filter_name in &["empty", "limit_10", "limit_100", "kinds_1"] {
        if let Some(filter_name) = filter_names.iter().find(|f| f == filter_name) {
            let mut row = vec![Cell::new(filter_name)];
            for result in results {
                if let Some(filter_result) = result.filters.get(filter_name) {
                    row.push(Cell::new(&format!("{:.1}", filter_result.avg_events)));
                } else {
                    row.push(Cell::new("N/A"));
                }
            }
            event_table.add_row(Row::new(row));
        }
    }
    
    event_table.printstd();
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load config
    let config = match BenchmarkConfig::load() {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("Warning: Could not load config.yaml: {}", e);
            None
        }
    };

    println!("🚀 Cassette WASM Benchmark (Rust)");
    println!("   Cassettes: {}", args.cassettes.len());

    let mut results = Vec::new();

    for cassette_path in &args.cassettes {
        if !cassette_path.exists() {
            eprintln!("❌ Not found: {}", cassette_path.display());
            continue;
        }

        // Determine iterations for this cassette
        let iterations = if let Some(iters) = args.iterations {
            iters
        } else if let Some(ref cfg) = config {
            cfg.get_iterations_for_cassette(cassette_path)
        } else {
            100 // Default fallback
        };

        match benchmark_cassette(cassette_path, iterations, args.debug) {
            Ok(result) => results.push(result),
            Err(e) => eprintln!("❌ Error with {}: {}", cassette_path.display(), e),
        }
    }

    if !results.is_empty() {
        print_comparison_table(&results);

        if let Some(output_path) = args.output {
            let output = json!({
                "timestamp": Utc::now().timestamp(),
                "iterations": args.iterations,
                "results": results,
            });
            fs::write(&output_path, serde_json::to_string_pretty(&output)?)?;
            println!("\n💾 Results saved to: {}", output_path.display());
        }
    }

    Ok(())
}

// Add rand for generating test data
use rand;