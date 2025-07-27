#!/usr/bin/env python3
"""
Run benchmarks and output clean results to results/<lang>.txt
"""

import subprocess
import os
import sys
import re
from pathlib import Path

def extract_python_results(output):
    """Extract clean Python benchmark results"""
    lines = output.split('\n')
    in_table = False
    results = []
    
    for line in lines:
        # Start capturing at performance comparison
        if 'CASSETTE PERFORMANCE COMPARISON' in line:
            in_table = True
            
        if in_table:
            # Stop at event retrieval stats
            if 'Event retrieval stats' in line:
                break
                
            # Keep relevant lines
            if any(pattern in line for pattern in [
                'CASSETTE PERFORMANCE', 'REQ QUERY PERFORMANCE', 'SUMMARY STATISTICS',
                'Filter Type', 'Cassette', '─', '═',
                'empty', 'limit_', 'kinds_', 'author', 'since', 'until', 
                'time_range', 'tag_', 'complex', '.wasm'
            ]) or re.match(r'^\s*\d+\.\d+', line):
                results.append(line)
    
    return '\n'.join(results)

def extract_rust_results(output):
    """Extract clean Rust benchmark results"""
    lines = output.split('\n')
    in_table = False
    results = []
    
    for line in lines:
        if 'Benchmark Results' in line:
            in_table = True
            
        if in_table:
            if line.strip() == '':
                break
                
            if any(pattern in line for pattern in [
                'Benchmark Results', 'Cassette', '╔', '║', '╚', '═', '─',
                'empty', 'limit_', 'kinds_', 'author', 'since', 'until',
                'time_range', 'tag_', 'complex', '.wasm'
            ]) or re.match(r'^\s*\d+\.\d+', line):
                results.append(line)
    
    return '\n'.join(results)

def extract_generic_results(output):
    """Extract generic benchmark metrics"""
    lines = output.split('\n')
    results = []
    
    # Skip build/install lines
    skip_patterns = [
        'Installing', 'Downloading', 'Building', 'Compiling',
        'Warning', 'Error', 'mkdir', 'cd ', 'make', 'cargo',
        'npm', 'pip', 'go get', 'dart pub'
    ]
    
    for line in lines:
        # Skip unwanted lines
        if any(pattern in line for pattern in skip_patterns):
            continue
            
        # Keep lines with performance metrics
        if any(pattern in line.lower() for pattern in [
            'ms', 'milliseconds', 'seconds', 'ops/sec', 'events/sec',
            'throughput', 'latency', 'p50', 'p95', 'p99',
            'average', 'median', 'benchmark', 'performance'
        ]):
            results.append(line)
    
    return '\n'.join(results)

def run_benchmark(lang, script_dir):
    """Run benchmark for a language and extract clean results"""
    lang_dir = os.path.join(script_dir, lang)
    
    if not os.path.exists(lang_dir):
        print(f"⚠️  Directory not found: {lang_dir}")
        return False
        
    if not os.path.exists(os.path.join(lang_dir, 'Makefile')):
        print(f"⚠️  No Makefile found in {lang_dir}")
        return False
    
    print(f"🚀 Running {lang} benchmark...")
    
    # Run the benchmark
    try:
        result = subprocess.run(
            ['make', 'run'],
            cwd=lang_dir,
            capture_output=True,
            text=True,
            timeout=300  # 5 minute timeout
        )
        
        output = result.stdout + '\n' + result.stderr
        
        # Extract clean results based on language
        if lang == 'py':
            clean_output = extract_python_results(output)
        elif lang == 'rust':
            clean_output = extract_rust_results(output)
        else:
            clean_output = extract_generic_results(output)
        
        # Save to results file
        results_dir = os.path.join(script_dir, 'results')
        os.makedirs(results_dir, exist_ok=True)
        
        output_file = os.path.join(results_dir, f'{lang}.txt')
        with open(output_file, 'w') as f:
            f.write(clean_output)
        
        if clean_output.strip():
            print(f"✅ {lang} benchmark completed")
            print(f"   Results saved to: {output_file}")
            return True
        else:
            print(f"⚠️  {lang} benchmark produced no results")
            with open(output_file, 'w') as f:
                f.write(f"No benchmark results found for {lang}\n")
            return False
            
    except subprocess.TimeoutExpired:
        print(f"❌ {lang} benchmark timed out")
        return False
    except Exception as e:
        print(f"❌ {lang} benchmark failed: {e}")
        return False

def main():
    if len(sys.argv) < 2:
        print("Usage: run_clean_benchmark.py <language> [language ...]")
        print("Available languages: py, rust, go, js, cpp, dart, deck")
        sys.exit(1)
    
    script_dir = Path(__file__).parent.parent
    languages = sys.argv[1:]
    
    if languages == ['all']:
        languages = ['py', 'rust', 'go', 'js', 'cpp', 'dart', 'deck']
    
    print("🎯 Running Clean Benchmarks")
    print("=" * 40)
    
    successful = 0
    failed = 0
    
    for lang in languages:
        if run_benchmark(lang, script_dir):
            successful += 1
        else:
            failed += 1
    
    print("\n📊 Summary")
    print("=" * 40)
    print(f"Successful: {successful}")
    print(f"Failed: {failed}")
    
    # Clean up old log files
    results_dir = os.path.join(script_dir, 'results')
    for pattern in ['*_output_*.log', 'benchmark_summary_*.txt']:
        for file in Path(results_dir).glob(pattern):
            file.unlink()
            print(f"🧹 Cleaned up: {file.name}")

if __name__ == '__main__':
    main()