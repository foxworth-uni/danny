# Danny vs Knip Benchmark Guide

This guide will help you run comprehensive benchmarks comparing Danny with Knip on the Next.js test application.

## Prerequisites

1. **Build Danny** (if not already built):
   ```bash
   cd /Users/fox/src/nine-gen/danny
   cargo build --release
   ```

2. **Install benchmark dependencies**:
   ```bash
   cd packages/benchmark
   pnpm install
   ```

3. **Install test app dependencies** (optional, but recommended for Knip):
   ```bash
   cd ../../test-files/nextjs-app
   pnpm install
   ```

## Quick Start

### Run Full Benchmark

```bash
cd packages/benchmark
pnpm benchmark
```

This will:
- Run both Danny and Knip 5 times each (with 1 warmup run)
- Measure execution time, memory usage, and findings
- Display a comparison table
- Save results to `./results/`

### View Results

```bash
# See comparison table in terminal
pnpm benchmark

# Generate HTML report
pnpm benchmark --format html

# Compare historical runs
pnpm benchmark:compare
```

## Detailed Usage

### Custom Benchmark Options

```bash
# Run more iterations for statistical accuracy
pnpm benchmark --iterations 10

# Run on different target
pnpm benchmark --target /path/to/project

# Run only Danny
pnpm benchmark --tools danny

# Run only Knip
pnpm benchmark --tools knip

# Generate HTML report
pnpm benchmark --format html

# Custom output directory
pnpm benchmark --output ./my-results
```

### Run Individual Tools

```bash
# Run Danny only
pnpm benchmark:danny

# Run Knip only
pnpm benchmark:knip
```

### Compare Results Over Time

```bash
# View historical benchmark data
pnpm benchmark:compare
```

This shows:
- Last 10 benchmark runs
- Performance trends (faster/slower)
- Memory trends (more/less memory)

## Understanding the Metrics

### Performance Metrics

- **Execution Time**: How long the tool takes to analyze the codebase
  - **Avg**: Average time across all iterations
  - **Min**: Fastest run
  - **Max**: Slowest run
  - **Std Dev**: Consistency (lower is better)

- **Memory Usage**: Peak memory consumption during analysis
  - **Peak**: Maximum memory used
  - **Avg**: Average memory across samples

### Accuracy Metrics

The benchmark uses `ground-truth.json` to measure accuracy:

- **True Positives (TP)**: Correctly identified dead code
- **False Positives (FP)**: Incorrectly flagged as dead code
- **False Negatives (FN)**: Missed dead code
- **Precision**: TP / (TP + FP) - How many findings are correct
- **Recall**: TP / (TP + FN) - How much dead code was found
- **F1 Score**: Harmonic mean of precision and recall

### What to Look For

**Speed Winner**: Lower execution time is better
- Danny is expected to be significantly faster due to Rust implementation

**Memory Winner**: Lower peak memory is better
- Native code typically uses less memory than Node.js

**Accuracy Winner**: Higher F1 score is better
- Both tools should have high precision
- Recall may vary based on detection capabilities

## Example Output

```
📊 Benchmark Results

┌─────────────────┬─────────────┬─────────────┐
│ Metric          │ danny       │ knip        │
├─────────────────┼─────────────┼─────────────┤
│ Avg Time        │ 45.23ms     │ 1,234.56ms  │
│ Min Time        │ 42.10ms     │ 1,198.23ms  │
│ Max Time        │ 48.91ms     │ 1,289.45ms  │
│ Std Dev         │ 2.34ms      │ 34.12ms     │
│ Peak Memory     │ 12.45 MB    │ 89.23 MB    │
│ Avg Memory      │ 10.23 MB    │ 78.45 MB    │
│ Findings        │ 6           │ 8           │
│ Success Rate    │ 100%        │ 100%        │
└─────────────────┴─────────────┴─────────────┘

⚡ Performance Comparison

🏆 Fastest: danny (45.23ms)
   knip: 1.23s (2630.0% slower)

💾 Least Memory: danny (10.23 MB)
   knip: 78.45 MB (+68.22 MB)

✨ Summary

🏆 Winner: danny
   Time: 45.23ms
   Memory: 10.23 MB
   Findings: 6
```

## Ground Truth

The benchmark uses `test-files/nextjs-app/ground-truth.json` to validate findings:

### Known Dead Code
- `components/Footer.jsx` - Never used
- `components/Card.jsx` - Never used
- `components/Sidebar.jsx` - Never used
- `components/UnusedModal.tsx` - Never used
- `lib/utils.js::unusedHelper` - Exported but not imported
- `lib/api.ts::deprecatedFetch` - Exported but not used

### Known Live Code
- `components/Header.jsx` - Used in `_app.jsx`
- `components/Button.tsx` - Used in multiple pages
- `lib/utils.js::formatDate` - Used in `index.jsx`
- `lib/api.ts::fetchData` - Used in `demo.tsx`

## Troubleshooting

### Danny Not Found

```bash
# Build Danny first
cd ../..
cargo build --release
```

### Knip Fails

```bash
# Install dependencies in test app
cd ../../test-files/nextjs-app
pnpm install
```

### Permission Errors

```bash
# Make sure Danny is executable
chmod +x ../../target/release/danny
```

### Out of Memory

```bash
# Reduce iterations
pnpm benchmark --iterations 3
```

## Advanced Usage

### Custom Ground Truth

Edit `test-files/nextjs-app/ground-truth.json` to add your own test cases:

```json
{
  "unused": {
    "components": [
      {
        "file": "components/MyComponent.jsx",
        "exports": ["MyComponent"],
        "reason": "Never imported"
      }
    ]
  }
}
```

### Add More Tools

Create a new runner in `src/runners/`:

```javascript
// src/runners/mytool.js
export async function runMyTool(target, options = {}) {
  // Implementation
}
```

Then add to `src/benchmark.js`.

### Export Results

```bash
# Results are automatically saved as JSON
cat results/benchmark-*.json | jq .
```

## CI/CD Integration

Add to your CI pipeline:

```yaml
- name: Benchmark Danny
  run: |
    cd packages/benchmark
    pnpm install
    pnpm benchmark --iterations 3 --format json
```

## Tips for Accurate Benchmarks

1. **Close other applications** to reduce noise
2. **Run multiple iterations** (5-10) for statistical significance
3. **Use warmup runs** to eliminate cold start effects
4. **Run on consistent hardware** for comparing over time
5. **Check system load** before benchmarking

## Next Steps

- Run benchmarks on larger codebases
- Test with different project types (React, Vue, etc.)
- Compare with other tools (depcheck, unimported, etc.)
- Contribute improvements to the benchmark suite

## Questions?

Check the main README or open an issue on GitHub.

