package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"io/ioutil"
	"log"
	"math"
	"math/rand"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	cassette "github.com/cassette-test/bindings/go"
)

// Generate random hex string
func generateRandomHex(length int) string {
	const hexChars = "0123456789abcdef"
	result := make([]byte, length)
	for i := 0; i < length; i++ {
		result[i] = hexChars[rand.Intn(16)]
	}
	return string(result)
}

// TestFilter represents a test filter configuration
type TestFilter struct {
	Name   string
	Filter map[string]interface{}
}

// Generate test filters
func generateTestFilters() []TestFilter {
	filters := []TestFilter{}
	now := time.Now().Unix()

	filters = append(filters, TestFilter{"empty", map[string]interface{}{}})
	filters = append(filters, TestFilter{"limit_1", map[string]interface{}{"limit": 1}})
	filters = append(filters, TestFilter{"limit_10", map[string]interface{}{"limit": 10}})
	filters = append(filters, TestFilter{"limit_100", map[string]interface{}{"limit": 100}})
	filters = append(filters, TestFilter{"limit_1000", map[string]interface{}{"limit": 1000}})
	filters = append(filters, TestFilter{"kinds_1", map[string]interface{}{"kinds": []int{1}}})
	filters = append(filters, TestFilter{"kinds_multiple", map[string]interface{}{"kinds": []int{1, 7, 0}}})

	filters = append(filters, TestFilter{"author_single", map[string]interface{}{
		"authors": []string{generateRandomHex(64)},
	}})

	authors := make([]string, 5)
	for i := 0; i < 5; i++ {
		authors[i] = generateRandomHex(64)
	}
	filters = append(filters, TestFilter{"authors_5", map[string]interface{}{"authors": authors}})

	filters = append(filters, TestFilter{"since_recent", map[string]interface{}{"since": now - 3600}})
	filters = append(filters, TestFilter{"until_now", map[string]interface{}{"until": now}})
	filters = append(filters, TestFilter{"time_range", map[string]interface{}{
		"since": now - 86400,
		"until": now,
	}})

	filters = append(filters, TestFilter{"tag_e", map[string]interface{}{"#e": []string{generateRandomHex(64)}}})
	filters = append(filters, TestFilter{"tag_p", map[string]interface{}{"#p": []string{generateRandomHex(64)}}})

	filters = append(filters, TestFilter{"complex", map[string]interface{}{
		"kinds":   []int{1},
		"limit":   50,
		"since":   now - 86400,
		"authors": []string{generateRandomHex(64)},
	}})

	return filters
}

// BenchmarkResult stores benchmark results
type BenchmarkResult struct {
	CassetteName      string
	FileSize          int64
	EventCount        int
	FilterTimings     map[string][]float64
	FilterEventCounts map[string][]int
}

// Calculate percentile
func percentile(values []float64, p float64) float64 {
	if len(values) == 0 {
		return 0.0
	}
	sorted := make([]float64, len(values))
	copy(sorted, values)
	sort.Float64s(sorted)
	index := int(float64(len(sorted)) * p)
	if index >= len(sorted) {
		index = len(sorted) - 1
	}
	return sorted[index]
}

// Calculate average
func average(values []float64) float64 {
	if len(values) == 0 {
		return 0.0
	}
	sum := 0.0
	for _, v := range values {
		sum += v
	}
	return sum / float64(len(values))
}

// Calculate int average
func averageInt(values []int) float64 {
	if len(values) == 0 {
		return 0.0
	}
	sum := 0
	for _, v := range values {
		sum += v
	}
	return float64(sum) / float64(len(values))
}

// Benchmark a single cassette
func benchmarkCassette(cassettePath string, iterations int) (*BenchmarkResult, error) {
	fmt.Printf("\n📼 Benchmarking: %s\n", filepath.Base(cassettePath))
	fmt.Println(strings.Repeat("=", 60))

	fileInfo, err := os.Stat(cassettePath)
	if err != nil {
		return nil, err
	}

	result := &BenchmarkResult{
		CassetteName:      filepath.Base(cassettePath),
		FileSize:          fileInfo.Size(),
		FilterTimings:     make(map[string][]float64),
		FilterEventCounts: make(map[string][]int),
	}

	// Load cassette
	c, err := cassette.Load(cassettePath, false)
	if err != nil {
		return nil, err
	}

	// Get cassette info
	infoStr := c.Info()
	var info map[string]interface{}
	if err := json.Unmarshal([]byte(infoStr), &info); err != nil {
		return nil, err
	}

	if eventCount, ok := info["event_count"].(float64); ok {
		result.EventCount = int(eventCount)
	}

	fmt.Printf("ℹ️  Cassette: %v\n", info["name"])
	fmt.Printf("   Events: %d\n", result.EventCount)
	fmt.Printf("   Size: %.1f KB\n", float64(result.FileSize)/1024)

	// Warm up
	fmt.Println("🔥 Warming up...")
	for i := 0; i < 10; i++ {
		req := []interface{}{"REQ", fmt.Sprintf("warmup-%d", i), map[string]interface{}{"limit": 1}}
		reqBytes, _ := json.Marshal(req)
		c.Send(string(reqBytes))
	}

	// Test filters
	testFilters := generateTestFilters()

	fmt.Printf("\n🏃 Running %d iterations per filter...\n", iterations)

	for idx, test := range testFilters {
		fmt.Printf("\n  Testing filter %d/%d: %s", idx+1, len(testFilters), test.Name)

		times := make([]float64, 0, iterations)
		eventCounts := make([]int, 0, iterations)

		for i := 0; i < iterations; i++ {
			if i%10 == 0 {
				fmt.Print(".")
			}

			subId := fmt.Sprintf("bench-%s-%d", test.Name, i)
			req := []interface{}{"REQ", subId, test.Filter}
			reqBytes, _ := json.Marshal(req)

			start := time.Now()
			response := c.Send(string(reqBytes))
			elapsed := time.Since(start).Seconds() * 1000 // Convert to milliseconds

			times = append(times, elapsed)

			// Count events returned
			eventCount := 0
			if !response.IsSingle && response.Multiple != nil {
				for _, msg := range response.Multiple {
					var parsed []interface{}
					if err := json.Unmarshal([]byte(msg), &parsed); err == nil {
						if len(parsed) > 0 && parsed[0] == "EVENT" {
							eventCount++
						}
					}
				}
			}
			eventCounts = append(eventCounts, eventCount)
		}

		result.FilterTimings[test.Name] = times
		result.FilterEventCounts[test.Name] = eventCounts

		avgMs := average(times)
		avgEvents := averageInt(eventCounts)

		fmt.Printf(" ✓ (%.1fms avg, %.0f events)\n", avgMs, avgEvents)
	}

	return result, nil
}

// Print comparison table
func printComparisonTable(results []*BenchmarkResult) {
	fmt.Println("\n📊 CASSETTE PERFORMANCE COMPARISON")
	fmt.Println(strings.Repeat("=", 100))

	fmt.Println("\n🔍 REQ QUERY PERFORMANCE (milliseconds)")
	fmt.Println(strings.Repeat("=", 100))

	// Collect all filter names
	filterSet := make(map[string]bool)
	for _, result := range results {
		for filterName := range result.FilterTimings {
			filterSet[filterName] = true
		}
	}

	filterNames := make([]string, 0, len(filterSet))
	for name := range filterSet {
		filterNames = append(filterNames, name)
	}
	sort.Strings(filterNames)

	// Print header
	fmt.Printf("%-20s", "Filter Type")
	for _, result := range results {
		name := result.CassetteName
		if len(name) > 12 {
			name = name[:12]
		}
		fmt.Printf("%12s ", name)
	}
	fmt.Println()
	fmt.Println(strings.Repeat("-", 20+13*len(results)))

	// Print data
	for _, filterName := range filterNames {
		fmt.Printf("%-20s", filterName)
		for _, result := range results {
			if times, ok := result.FilterTimings[filterName]; ok && len(times) > 0 {
				avgMs := average(times)
				fmt.Printf("%11.2f  ", avgMs)
			} else {
				fmt.Printf("%11s  ", "N/A")
			}
		}
		fmt.Println()
	}

	// Summary stats
	fmt.Println("\n📈 SUMMARY STATISTICS")
	fmt.Println(strings.Repeat("=", 100))
	fmt.Printf("%-30s %10s %10s %10s %10s\n", "Cassette", "Size (KB)", "Events", "Avg (ms)", "P95 (ms)")
	fmt.Println(strings.Repeat("-", 70))

	for _, result := range results {
		allTimes := []float64{}
		for _, times := range result.FilterTimings {
			allTimes = append(allTimes, times...)
		}

		if len(allTimes) > 0 {
			avgTime := average(allTimes)
			p95 := percentile(allTimes, 0.95)

			fmt.Printf("%-30s %10.1f %10d %10.2f %10.2f\n",
				result.CassetteName,
				float64(result.FileSize)/1024,
				result.EventCount,
				avgTime,
				p95)
		}
	}
}

func main() {
	var iterations int
	flag.IntVar(&iterations, "iterations", 100, "Number of iterations per test")
	flag.IntVar(&iterations, "i", 100, "Number of iterations per test (shorthand)")
	flag.Parse()

	cassettePaths := flag.Args()
	if len(cassettePaths) == 0 {
		fmt.Fprintf(os.Stderr, "Usage: %s [--iterations N] <cassette.wasm> [cassette2.wasm ...]\n", os.Args[0])
		os.Exit(1)
	}

	// Initialize random seed
	rand.Seed(time.Now().UnixNano())

	fmt.Println("🚀 Cassette WASM Benchmark (Go)")
	fmt.Printf("   Cassettes: %d\n", len(cassettePaths))
	fmt.Printf("   Iterations: %d\n", iterations)

	results := []*BenchmarkResult{}

	for _, path := range cassettePaths {
		if _, err := os.Stat(path); os.IsNotExist(err) {
			fmt.Printf("❌ Not found: %s\n", path)
			continue
		}

		result, err := benchmarkCassette(path, iterations)
		if err != nil {
			fmt.Printf("❌ Error with %s: %v\n", path, err)
			continue
		}

		results = append(results, result)
	}

	if len(results) > 0 {
		printComparisonTable(results)

		// Save results to JSON
		output := map[string]interface{}{
			"timestamp":  time.Now().Unix(),
			"iterations": iterations,
			"results":    []map[string]interface{}{},
		}

		for _, result := range results {
			cassResult := map[string]interface{}{
				"cassette":    result.CassetteName,
				"file_size":   result.FileSize,
				"event_count": result.EventCount,
				"filters":     map[string]interface{}{},
			}

			filters := cassResult["filters"].(map[string]interface{})
			for filterName, times := range result.FilterTimings {
				if len(times) > 0 {
					minTime := times[0]
					maxTime := times[0]
					for _, t := range times {
						if t < minTime {
							minTime = t
						}
						if t > maxTime {
							maxTime = t
						}
					}

					filterData := map[string]interface{}{
						"count":  len(times),
						"avg_ms": average(times),
						"min_ms": minTime,
						"max_ms": maxTime,
						"p50_ms": percentile(times, 0.5),
						"p95_ms": percentile(times, 0.95),
						"p99_ms": percentile(times, 0.99),
					}

					if eventCounts, ok := result.FilterEventCounts[filterName]; ok && len(eventCounts) > 0 {
						filterData["avg_events"] = averageInt(eventCounts)
						maxEvents := eventCounts[0]
						for _, e := range eventCounts {
							if e > maxEvents {
								maxEvents = e
							}
						}
						filterData["max_events"] = maxEvents
					}

					filters[filterName] = filterData
				}
			}

			output["results"] = append(output["results"].([]map[string]interface{}), cassResult)
		}

		outputFilename := fmt.Sprintf("benchmark_go_%d.json", time.Now().Unix())
		outputBytes, _ := json.MarshalIndent(output, "", "  ")
		if err := ioutil.WriteFile(outputFilename, outputBytes, 0644); err != nil {
			log.Printf("Failed to save results: %v", err)
		} else {
			fmt.Printf("\n💾 Results saved to: %s\n", outputFilename)
		}
	}
}