# Quick Start - Danny Benchmark

Get up and running in 3 minutes!

## Setup (One Time)

```bash
cd packages/benchmark
./setup.sh
```

This will:
- ✓ Build Danny
- ✓ Install dependencies
- ✓ Set up test environment
- ✓ Verify everything works

## Run Benchmark

```bash
pnpm benchmark
```

That's it! You'll see:
- Performance comparison table
- Execution time (Danny vs Knip)
- Memory usage
- Findings count
- Winner announcement 🏆

## What You'll See

```
🚀 Danny Benchmark Suite

Configuration:
  Target: ../../test-files/nextjs-app
  Iterations: 5
  Warmup: 1
  Tools: danny, knip

📊 Running Benchmarks

▶ Benchmarking danny...
✓ danny: Completed 5 iterations

▶ Benchmarking knip...
✓ knip: Completed 5 iterations

📊 Benchmark Results

┌─────────────────┬─────────────┬─────────────┐
│ Metric          │ danny       │ knip        │
├─────────────────┼─────────────┼─────────────┤
│ Avg Time        │ 45.23ms     │ 1,234.56ms  │
│ Peak Memory     │ 12.45 MB    │ 89.23 MB    │
│ Findings        │ 6           │ 8           │
└─────────────────┴─────────────┴─────────────┘

⚡ Performance Comparison

🏆 Fastest: danny (45.23ms)
   knip: 1.23s (2630% slower)

💾 Least Memory: danny (10.23 MB)
   knip: 78.45 MB (+68.22 MB)

✨ Summary

🏆 Winner: danny
```

## Common Commands

```bash
# Basic benchmark
pnpm benchmark

# More iterations (more accurate)
pnpm benchmark --iterations 10

# HTML report
pnpm benchmark --format html

# Compare historical runs
pnpm benchmark:compare

# Run only Danny
pnpm benchmark --tools danny

# Run only Knip
pnpm benchmark --tools knip

# Help
pnpm benchmark --help
```

## Troubleshooting

### "Danny not found"
```bash
cd ../..
cargo build --release
```

### "Knip fails"
```bash
cd ../../test-files/nextjs-app
pnpm install
```

### "Permission denied"
```bash
chmod +x ../../target/release/danny
```

## Next Steps

- Read [BENCHMARK_GUIDE.md](./BENCHMARK_GUIDE.md) for detailed documentation
- Check [README.md](./README.md) for architecture details
- View results in `./results/` directory

## Expected Results

Danny should be:
- **20-30x faster** than Knip
- **5-10x less memory** usage
- **Similar or better accuracy** (F1 score)

## Questions?

See the full [BENCHMARK_GUIDE.md](./BENCHMARK_GUIDE.md) or open an issue.

