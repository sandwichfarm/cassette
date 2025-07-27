import 'dart:io';
import 'dart:convert';
import 'dart:math';
import 'package:cassette_loader/cassette_loader.dart';

// Generate random hex string
String generateRandomHex(int length) {
  final random = Random();
  return List.generate(length, (_) => random.nextInt(16).toRadixString(16)).join();
}

// Test filter configuration
class TestFilter {
  final String name;
  final Map<String, dynamic> filter;
  
  TestFilter(this.name, this.filter);
}

// Generate test filters
List<TestFilter> generateTestFilters() {
  final filters = <TestFilter>[];
  final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
  
  filters.add(TestFilter('empty', {}));
  filters.add(TestFilter('limit_1', {'limit': 1}));
  filters.add(TestFilter('limit_10', {'limit': 10}));
  filters.add(TestFilter('limit_100', {'limit': 100}));
  filters.add(TestFilter('limit_1000', {'limit': 1000}));
  filters.add(TestFilter('kinds_1', {'kinds': [1]}));
  filters.add(TestFilter('kinds_multiple', {'kinds': [1, 7, 0]}));
  
  filters.add(TestFilter('author_single', {'authors': [generateRandomHex(64)]}));
  
  final authors = List.generate(5, (_) => generateRandomHex(64));
  filters.add(TestFilter('authors_5', {'authors': authors}));
  
  filters.add(TestFilter('since_recent', {'since': now - 3600}));
  filters.add(TestFilter('until_now', {'until': now}));
  filters.add(TestFilter('time_range', {'since': now - 86400, 'until': now}));
  
  filters.add(TestFilter('tag_e', {'#e': [generateRandomHex(64)]}));
  filters.add(TestFilter('tag_p', {'#p': [generateRandomHex(64)]}));
  
  filters.add(TestFilter('complex', {
    'kinds': [1],
    'limit': 50,
    'since': now - 86400,
    'authors': [generateRandomHex(64)]
  }));
  
  return filters;
}

// Benchmark result
class BenchmarkResult {
  final String cassetteName;
  final int fileSize;
  final int eventCount;
  final Map<String, List<double>> filterTimings = {};
  final Map<String, List<int>> filterEventCounts = {};
  
  BenchmarkResult(this.cassetteName, this.fileSize, this.eventCount);
}

// Calculate percentile
double percentile(List<double> values, double p) {
  if (values.isEmpty) return 0.0;
  final sorted = List<double>.from(values)..sort();
  final index = (sorted.length * p).floor();
  return sorted[min(index, sorted.length - 1)];
}

// Calculate average
double average(List<num> values) {
  if (values.isEmpty) return 0.0;
  return values.reduce((a, b) => a + b) / values.length;
}

// Benchmark a single cassette
Future<BenchmarkResult> benchmarkCassette(String cassettePath, int iterations) async {
  print('\n📼 Benchmarking: ${File(cassettePath).uri.pathSegments.last}');
  print('=' * 60);
  
  final file = File(cassettePath);
  final fileSize = await file.length();
  final cassetteName = file.uri.pathSegments.last;
  
  try {
    // Load cassette
    final cassette = await Cassette.load(cassettePath, debug: false);
    
    // Get cassette info
    final infoStr = cassette.info();
    final info = jsonDecode(infoStr);
    
    final eventCount = info['event_count'] ?? 0;
    final result = BenchmarkResult(cassetteName, fileSize, eventCount);
    
    print('ℹ️  Cassette: ${info['name'] ?? 'Unknown'}');
    print('   Events: $eventCount');
    print('   Size: ${(fileSize / 1024).toStringAsFixed(1)} KB');
    
    // Warm up
    print('🔥 Warming up...');
    for (int i = 0; i < 10; i++) {
      final req = jsonEncode(['REQ', 'warmup-$i', {'limit': 1}]);
      cassette.send(req);
    }
    
    // Test filters
    final testFilters = generateTestFilters();
    
    print('\n🏃 Running $iterations iterations per filter...');
    
    for (int idx = 0; idx < testFilters.length; idx++) {
      final test = testFilters[idx];
      stdout.write('\n  Testing filter ${idx + 1}/${testFilters.length}: ${test.name}');
      
      final times = <double>[];
      final eventCounts = <int>[];
      
      for (int i = 0; i < iterations; i++) {
        if (i % 10 == 0) stdout.write('.');
        
        final subId = 'bench-${test.name}-$i';
        final reqMessage = jsonEncode(['REQ', subId, test.filter]);
        
        final stopwatch = Stopwatch()..start();
        final response = cassette.send(reqMessage);
        stopwatch.stop();
        
        times.add(stopwatch.elapsedMicroseconds / 1000.0);
        
        // Count events returned
        int eventCount = 0;
        if (response is List<String>) {
          for (final msg in response) {
            try {
              final parsed = jsonDecode(msg);
              if (parsed[0] == 'EVENT') eventCount++;
            } catch (_) {}
          }
        }
        eventCounts.add(eventCount);
      }
      
      result.filterTimings[test.name] = times;
      result.filterEventCounts[test.name] = eventCounts;
      
      final avgMs = average(times);
      final avgEvents = average(eventCounts);
      
      print(' ✓ (${avgMs.toStringAsFixed(1)}ms avg, ${avgEvents.toStringAsFixed(0)} events)');
    }
    
    return result;
    
  } catch (e) {
    print('❌ Error: $e');
    return BenchmarkResult(cassetteName, fileSize, 0);
  }
}

// Print comparison table
void printComparisonTable(List<BenchmarkResult> results) {
  print('\n📊 CASSETTE PERFORMANCE COMPARISON');
  print('=' * 100);
  
  print('\n🔍 REQ QUERY PERFORMANCE (milliseconds)');
  print('=' * 100);
  
  // Collect all filter names
  final allFilters = <String>{};
  for (final result in results) {
    allFilters.addAll(result.filterTimings.keys);
  }
  
  final filterNames = allFilters.toList()..sort();
  
  // Print header
  stdout.write('Filter Type'.padRight(20));
  for (final result in results) {
    final name = result.cassetteName.length > 12 
        ? result.cassetteName.substring(0, 12) 
        : result.cassetteName;
    stdout.write(name.padLeft(12) + ' ');
  }
  print('');
  print('-' * (20 + 13 * results.length));
  
  // Print data
  for (final filterName in filterNames) {
    stdout.write(filterName.padRight(20));
    for (final result in results) {
      if (result.filterTimings.containsKey(filterName)) {
        final avgMs = average(result.filterTimings[filterName]!);
        stdout.write(avgMs.toStringAsFixed(2).padLeft(11) + '  ');
      } else {
        stdout.write('N/A'.padLeft(11) + '  ');
      }
    }
    print('');
  }
  
  // Summary stats
  print('\n📈 SUMMARY STATISTICS');
  print('=' * 100);
  print('${'Cassette'.padRight(30)} ${'Size (KB)'.padLeft(10)} '
        '${'Events'.padLeft(10)} ${'Avg (ms)'.padLeft(10)} ${'P95 (ms)'.padLeft(10)}');
  print('-' * 70);
  
  for (final result in results) {
    final allTimes = <double>[];
    for (final times in result.filterTimings.values) {
      allTimes.addAll(times);
    }
    
    if (allTimes.isNotEmpty) {
      final avgTime = average(allTimes);
      final p95 = percentile(allTimes, 0.95);
      
      print('${result.cassetteName.padRight(30)} '
            '${(result.fileSize / 1024).toStringAsFixed(1).padLeft(10)} '
            '${result.eventCount.toString().padLeft(10)} '
            '${avgTime.toStringAsFixed(2).padLeft(10)} '
            '${p95.toStringAsFixed(2).padLeft(10)}');
    }
  }
}

void main(List<String> args) async {
  if (args.isEmpty) {
    stderr.writeln('Usage: dart benchmark.dart <cassette.wasm> [cassette2.wasm ...]');
    exit(1);
  }
  
  int iterations = 100;
  final cassettePaths = <String>[];
  
  // Parse command line arguments
  for (int i = 0; i < args.length; i++) {
    if (args[i] == '--iterations' || args[i] == '-i') {
      if (i + 1 < args.length) {
        iterations = int.parse(args[++i]);
      }
    } else {
      cassettePaths.add(args[i]);
    }
  }
  
  if (cassettePaths.isEmpty) {
    stderr.writeln('Error: No cassette files specified');
    exit(1);
  }
  
  print('🚀 Cassette WASM Benchmark (Dart)');
  print('   Cassettes: ${cassettePaths.length}');
  print('   Iterations: $iterations');
  
  final results = <BenchmarkResult>[];
  
  for (final path in cassettePaths) {
    if (!await File(path).exists()) {
      print('❌ Not found: $path');
      continue;
    }
    
    try {
      results.add(await benchmarkCassette(path, iterations));
    } catch (e) {
      print('❌ Error with $path: $e');
    }
  }
  
  if (results.isNotEmpty) {
    printComparisonTable(results);
    
    // Save results to JSON
    final output = {
      'timestamp': DateTime.now().millisecondsSinceEpoch ~/ 1000,
      'iterations': iterations,
      'results': results.map((result) => {
        'cassette': result.cassetteName,
        'file_size': result.fileSize,
        'event_count': result.eventCount,
        'filters': result.filterTimings.map((filterName, times) => MapEntry(
          filterName,
          {
            'count': times.length,
            'avg_ms': average(times),
            'min_ms': times.reduce(min),
            'max_ms': times.reduce(max),
            'p50_ms': percentile(times, 0.5),
            'p95_ms': percentile(times, 0.95),
            'p99_ms': percentile(times, 0.99),
            'avg_events': result.filterEventCounts[filterName] != null 
                ? average(result.filterEventCounts[filterName]!) 
                : 0,
            'max_events': result.filterEventCounts[filterName] != null 
                ? result.filterEventCounts[filterName]!.reduce(max) 
                : 0,
          }
        ))
      }).toList()
    };
    
    final outputFilename = 'benchmark_dart_${DateTime.now().millisecondsSinceEpoch ~/ 1000}.json';
    await File(outputFilename).writeAsString(jsonEncode(output));
    
    print('\n💾 Results saved to: $outputFilename');
  }
}