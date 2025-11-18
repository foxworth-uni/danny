# Danny Benchmarking - Complete Overview

This document provides a comprehensive overview of the Danny benchmark suite for comparing dead code detection tools.

## 📁 Package Structure

```
packages/benchmark/
├── src/
│   ├── index.js           # Main CLI entry point
│   ├── benchmark.js       # Core benchmarking logic
│   ├── report.js          # Report generation
│   ├── compare.js         # Historical comparison
│   ├── accuracy.js        # Accuracy metrics calculation
│   └── runners/
│       ├── danny.js       # Danny runner
│       └── knip.js        # Knip runner
├── results/               # Benchmark results (auto-generated)
├── package.json
├── README.md              # Package overview
├── BENCHMARK_GUIDE.md     # Detailed guide
├── QUICK_START.md         # Quick start guide
└── setup.sh               # Setup script
```

## 🎯 What Gets Benchmarked

### Performance Metrics

1. **Execution Time**
   - Total time from start to finish
   - Measured using high-resolution timers
   - Statistics: min, max, avg, median, stddev
   - Multiple iterations for accuracy

2. **Memory Usage**
   - Peak memory consumption (RSS)
   - Average memory during execution
   - Sampled every 100ms
   - Measured in MB

3. **Reliability**
   - Success rate across iterations
   - Error handling
   - Consistency of results

### Accuracy Metrics

Based on `ground-truth.json`:

1. **True Positives (TP)**
   - Dead code correctly identified
   - Matches expected unused exports

2. **False Positives (FP)**
   - Live code incorrectly flagged
   - Matches expected used exports

3. **False Negatives (FN)**
   - Dead code that was missed
   - Expected unused but not found

4. **Derived Metrics**
   - **Precision**: TP / (TP + FP)
   - **Recall**: TP / (TP + FN)
   - **F1 Score**: 2 × (Precision × Recall) / (Precision + Recall)

## 🧪 Test Case: Next.js App

The benchmark uses a realistic Next.js application with:

### Known Dead Code
- 4 unused components (Footer, Card, Sidebar, UnusedModal)
- 2 unused functions (unusedHelper, deprecatedFetch)
- 1 unused variable (UNUSED_CONSTANT)
- Potentially unused dependencies

### Known Live Code
- 2 used components (Header, Button)
- 2 used functions (formatDate, fetchData)
- All page files
- Framework files (_app, _document)

### Why This Test Case?

1. **Realistic**: Actual Next.js app structure
2. **Runnable**: App works perfectly without dead code
3. **Diverse**: Mix of .js, .jsx, .ts, .tsx files
4. **Framework-aware**: Tests framework-specific patterns
5. **Ground truth**: Known correct answers for validation

## 🔬 Methodology

### Benchmark Process

1. **Warmup Phase**
   - Run each tool once to warm up caches
   - Not counted in statistics
   - Eliminates cold start bias

2. **Measurement Phase**
   - Run each tool N times (default: 5)
   - Measure time and memory for each run
   - Collect all findings

3. **Analysis Phase**
   - Calculate statistics (avg, min, max, stddev)
   - Compare against ground truth
   - Generate accuracy metrics

4. **Reporting Phase**
   - Display comparison table
   - Show performance comparison
   - Identify winner
   - Save results to JSON

### Statistical Rigor

- **Multiple iterations**: Reduces random variation
- **Warmup runs**: Eliminates cold start effects
- **Standard deviation**: Measures consistency
- **Median calculation**: Robust against outliers
- **High-resolution timers**: Microsecond precision

## 📊 Expected Results

Based on initial testing:

### Danny (Rust)
- **Speed**: ~40-60ms
- **Memory**: ~10-15 MB
- **Findings**: 6-8 items
- **Accuracy**: High precision, good recall

### Knip (Node.js)
- **Speed**: ~1,000-1,500ms
- **Memory**: ~70-100 MB
- **Findings**: 6-10 items
- **Accuracy**: High precision, good recall

### Comparison
- **Danny is ~20-30x faster**
- **Danny uses ~5-10x less memory**
- **Similar accuracy** (both are good)

## 🚀 Usage Patterns

### Quick Benchmark
```bash
pnpm benchmark
```
Best for: Quick comparison, CI/CD

### Detailed Benchmark
```bash
pnpm benchmark --iterations 10 --format html
```
Best for: Detailed analysis, presentations

### Historical Comparison
```bash
pnpm benchmark:compare
```
Best for: Tracking performance over time

### Custom Target
```bash
pnpm benchmark --target /path/to/project
```
Best for: Testing on different codebases

## 🎨 Output Formats

### Table (Default)
- Console-friendly
- Easy to read
- Color-coded
- Perfect for terminal

### JSON
- Machine-readable
- Full details
- Programmatic access
- CI/CD integration

### HTML
- Beautiful reports
- Charts and graphs
- Shareable
- Presentations

## 🔧 Extensibility

### Adding New Tools

1. Create runner in `src/runners/mytool.js`:
```javascript
export async function runMyTool(target, options = {}) {
  // Run tool
  // Return findings
}
```

2. Add to `src/benchmark.js`:
```javascript
if (tool === 'mytool') {
  result = await runMyTool(target);
}
```

3. Update config to include tool:
```bash
pnpm benchmark --tools danny,knip,mytool
```

### Adding New Metrics

1. Extend `ResourceMonitor` in `benchmark.js`
2. Add to statistics calculation
3. Update report generation
4. Document in guide

### Custom Ground Truth

Edit `test-files/nextjs-app/ground-truth.json`:
```json
{
  "unused": {
    "components": [
      { "file": "...", "exports": [...] }
    ]
  }
}
```

## 📈 CI/CD Integration

### GitHub Actions Example

```yaml
name: Benchmark
on: [push, pull_request]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: pnpm/action-setup@v2
      - uses: actions/setup-node@v3
      - uses: actions-rs/toolchain@v1
      
      - name: Build Danny
        run: cargo build --release
      
      - name: Run Benchmark
        run: |
          cd packages/benchmark
          pnpm install
          pnpm benchmark --iterations 3 --format json
      
      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: packages/benchmark/results/
```

## 🎯 Best Practices

### For Accurate Benchmarks

1. **Close other applications**
2. **Run multiple iterations** (5-10)
3. **Use warmup runs**
4. **Consistent hardware**
5. **Check system load**
6. **Same Node.js version**
7. **Same Rust version**

### For Comparisons

1. **Same test case**
2. **Same configuration**
3. **Same environment**
4. **Multiple runs**
5. **Statistical analysis**

### For Reporting

1. **Include context** (hardware, versions)
2. **Show variance** (not just average)
3. **Explain methodology**
4. **Provide raw data**
5. **Be honest** about limitations

## 🔍 Interpreting Results

### Speed Comparison

- **<10% difference**: Roughly equivalent
- **10-50% difference**: Noticeably faster
- **50-200% difference**: Significantly faster
- **>200% difference**: Dramatically faster

### Memory Comparison

- **<20% difference**: Similar
- **20-100% difference**: Noticeable
- **>100% difference**: Significant

### Accuracy Comparison

- **F1 > 0.95**: Excellent
- **F1 > 0.90**: Very good
- **F1 > 0.80**: Good
- **F1 > 0.70**: Acceptable
- **F1 < 0.70**: Needs improvement

## 🐛 Troubleshooting

### Common Issues

1. **Danny not found**
   - Solution: `cargo build --release`

2. **Knip fails**
   - Solution: `cd test-files/nextjs-app && pnpm install`

3. **Out of memory**
   - Solution: Reduce iterations

4. **Inconsistent results**
   - Solution: Close other apps, increase iterations

5. **Permission errors**
   - Solution: `chmod +x target/release/danny`

## 📚 Further Reading

- [QUICK_START.md](./QUICK_START.md) - Get started in 3 minutes
- [BENCHMARK_GUIDE.md](./BENCHMARK_GUIDE.md) - Detailed usage guide
- [README.md](./README.md) - Package overview
- [../test-files/nextjs-app/ground-truth.json](../../test-files/nextjs-app/ground-truth.json) - Test case details

## 🤝 Contributing

To improve the benchmark suite:

1. Add more test cases
2. Add more tools to compare
3. Improve accuracy metrics
4. Add visualization
5. Optimize performance
6. Improve documentation

## 📝 License

MIT - Same as Danny

