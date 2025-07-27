#!/usr/bin/env python3
"""
Load benchmark configuration from config.yaml
"""

import yaml
import os
import sys
import json
from pathlib import Path

def load_config(config_path=None):
    """Load configuration from YAML file"""
    if config_path is None:
        # Find config.yaml in benchmarks directory
        script_dir = Path(__file__).parent
        config_path = script_dir.parent / "config.yaml"
    
    if not config_path.exists():
        print(f"Error: Config file not found at {config_path}", file=sys.stderr)
        sys.exit(1)
    
    with open(config_path, 'r') as f:
        config = yaml.safe_load(f)
    
    return config

def get_iterations_for_cassette(cassette_path, config, language=None):
    """Get the number of iterations for a specific cassette based on its size"""
    if config is None:
        return 100  # Default if no config
        
    filename = os.path.basename(cassette_path).lower()
    
    # Check language-specific overrides first
    lang_overrides = config.get('language_overrides') or {}
    if language and language in lang_overrides:
        lang_config = config['language_overrides'][language]
        if 'iterations' in lang_config:
            # Check size patterns
            for size, patterns in config.get('size_patterns', {}).items():
                for pattern in patterns:
                    # Simple pattern matching (convert glob to basic matching)
                    pattern_check = pattern.replace('*', '')
                    if pattern_check in filename:
                        if size in lang_config['iterations']:
                            return lang_config['iterations'][size]
    
    # Check global iterations based on size patterns
    iterations = config.get('iterations', {})
    for size, patterns in config.get('size_patterns', {}).items():
        for pattern in patterns:
            # Simple pattern matching (convert glob to basic matching)
            pattern_check = pattern.replace('*', '')
            if pattern_check in filename:
                return iterations.get(size, config.get('default_iterations', 100))
    
    # Default fallback
    return config.get('default_iterations', 100)

def export_as_env_vars(config):
    """Export config as environment variables for shell scripts"""
    # Export iterations
    for size, iters in config.get('iterations', {}).items():
        env_var = f"BENCH_ITERATIONS_{size.upper()}"
        print(f'export {env_var}={iters}')
    
    print(f"export BENCH_DEFAULT_ITERATIONS={config.get('default_iterations', 100)}")
    print(f"export BENCH_WARMUP_ITERATIONS={config.get('test_config', {}).get('warmup_iterations', 10)}")

def main():
    """Main function for CLI usage"""
    import argparse
    
    parser = argparse.ArgumentParser(description='Load benchmark configuration')
    parser.add_argument('--config', help='Path to config file', default=None)
    parser.add_argument('--cassette', help='Get iterations for specific cassette')
    parser.add_argument('--language', help='Language for overrides (python, rust, etc.)')
    parser.add_argument('--export-env', action='store_true', help='Export as environment variables')
    parser.add_argument('--json', action='store_true', help='Output as JSON')
    
    args = parser.parse_args()
    
    config = load_config(args.config)
    
    if args.export_env:
        export_as_env_vars(config)
    elif args.cassette:
        iterations = get_iterations_for_cassette(args.cassette, config, args.language)
        print(iterations)
    elif args.json:
        print(json.dumps(config, indent=2))
    else:
        # Print summary
        print("Benchmark Configuration:")
        print(f"  Small cassettes: {config['iterations']['small']} iterations")
        print(f"  Medium cassettes: {config['iterations']['medium']} iterations")
        print(f"  Large cassettes: {config['iterations']['large']} iterations")

if __name__ == '__main__':
    main()