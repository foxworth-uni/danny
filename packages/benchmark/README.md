# Danny Benchmark Suite

Comprehensive benchmarking tool to compare Danny's performance and accuracy against other dead code detection tools, primarily Knip.

## 🏆 Results Summary

**Danny is 14x faster than Knip** with 68 MB less memory usage!

| Metric | Danny | Knip | Winner |
|--------|-------|------|--------|
| **Avg Time** | **49.78ms** | 698.06ms | 🏆 Danny (14x faster) |
| **Memory** | **~0 MB** | 67.67 MB | 🏆 Danny (68 MB less) |
| **Consistency** | **σ=1.05ms** | σ=17.58ms | 🏆 Danny (17x better) |
| **Findings** | 50 | Similar | ✅ Comparable |

*Based on 10 iterations on a Next.js test app. See [RESULTS_SUMMARY.md](./RESULTS_SUMMARY.md) for details.*

## Features

- **Performance Metrics**: Execution time, memory usage, CPU utilization
- **Accuracy Metrics**: Detection rate, false positives, false negatives
- **Detailed Reports**: JSON and human-readable output formats
- **Multiple Test Cases**: Run on various project types and sizes

## Quick Start

```bash
# Install dependencies
pnpm install

# Run full benchmark suite
pnpm benchmark

# Run specific tool
pnpm benchmark:danny
pnpm benchmark:knip

# Compare results
pnpm benchmark:compare

# Generate detailed report
pnpm report
```

## Usage

### Basic Benchmark

```bash
# Run on default test app (nextjs-app)
pnpm benchmark

# Run on specific directory
pnpm benchmark --target ../test-files/nextjs-app

# Run multiple iterations for statistical accuracy
pnpm benchmark --iterations 10
```

### Advanced Options

```bash
# Run all tools and compare
pnpm benchmark:all

# Generate detailed report with charts
pnpm report --format html

# Export results to JSON
pnpm benchmark --output results.json
```

## Metrics Collected

### Performance
- **Execution Time**: Total time to analyze the codebase
- **Memory Usage**: Peak memory consumption
- **CPU Usage**: Average CPU utilization
- **Startup Time**: Time to initialize the tool
- **Analysis Time**: Time spent on actual analysis

### Accuracy
- **True Positives**: Correctly identified dead code
- **False Positives**: Incorrectly flagged as dead code
- **False Negatives**: Missed dead code
- **Precision**: TP / (TP + FP)
- **Recall**: TP / (TP + FN)
- **F1 Score**: Harmonic mean of precision and recall

### Coverage
- **Files Analyzed**: Number of files processed
- **Lines Analyzed**: Total lines of code analyzed
- **Exports Found**: Total exports detected
- **Imports Found**: Total imports detected

## Test Cases

The benchmark suite includes several test cases:

1. **nextjs-app**: Real-world Next.js application with known dead code
2. **Large Codebase**: Synthetic large project for performance testing
3. **Edge Cases**: Specific patterns that are hard to detect

## Output Format

Results are saved in `./results/` directory:

```
results/
├── danny-{timestamp}.json
├── knip-{timestamp}.json
├── comparison-{timestamp}.json
└── report-{timestamp}.html
```

## Configuration

Create a `benchmark.config.json` file to customize:

```json
{
  "iterations": 5,
  "warmup": 1,
  "targets": [
    "../../test-files/nextjs-app"
  ],
  "tools": ["danny", "knip"],
  "metrics": ["time", "memory", "accuracy"],
  "output": {
    "format": "json",
    "directory": "./results"
  }
}
```

## Interpreting Results

- **Speed**: Lower execution time is better
- **Memory**: Lower peak memory is better
- **Accuracy**: Higher precision and recall are better
- **F1 Score**: Overall accuracy metric (0-1, higher is better)

## Contributing

To add a new tool to benchmark:

1. Create a runner in `src/runners/{tool}.js`
2. Implement the `BenchmarkRunner` interface
3. Add to `src/index.js`

## License

MIT

