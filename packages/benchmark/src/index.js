#!/usr/bin/env node

import { parseArgs } from 'node:util';
import { runBenchmark } from './benchmark.js';
import { generateReport } from './report.js';
import chalk from 'chalk';

const { values: args } = parseArgs({
  options: {
    target: {
      type: 'string',
      default: '../../test-files/nextjs-app',
    },
    iterations: {
      type: 'string',
      default: '5',
    },
    warmup: {
      type: 'string',
      default: '1',
    },
    tools: {
      type: 'string',
      default: 'danny,knip',
    },
    output: {
      type: 'string',
      default: './results',
    },
    format: {
      type: 'string',
      default: 'table',
    },
    all: {
      type: 'boolean',
      default: false,
    },
    help: {
      type: 'boolean',
      default: false,
    },
  },
});

if (args.help) {
  console.log(`
${chalk.bold('Danny Benchmark Suite')}

${chalk.dim('Usage:')}
  pnpm benchmark [options]

${chalk.dim('Options:')}
  --target <path>        Target directory to analyze (default: ../../test-files/nextjs-app)
  --iterations <n>       Number of iterations to run (default: 5)
  --warmup <n>          Number of warmup runs (default: 1)
  --tools <list>        Comma-separated list of tools (default: danny,knip)
  --output <path>       Output directory for results (default: ./results)
  --format <type>       Output format: table, json, html (default: table)
  --all                 Run all available tools
  --help                Show this help message

${chalk.dim('Examples:')}
  pnpm benchmark
  pnpm benchmark --iterations 10
  pnpm benchmark --target ../my-project
  pnpm benchmark --format json --output ./my-results
  `);
  process.exit(0);
}

async function main() {
  console.log(chalk.bold.cyan('\n🚀 Danny Benchmark Suite\n'));

  const config = {
    target: args.target,
    iterations: parseInt(args.iterations, 10),
    warmup: parseInt(args.warmup, 10),
    tools: args.all ? ['danny', 'knip'] : args.tools.split(','),
    output: args.output,
    format: args.format,
  };

  console.log(chalk.dim('Configuration:'));
  console.log(chalk.dim(`  Target: ${config.target}`));
  console.log(chalk.dim(`  Iterations: ${config.iterations}`));
  console.log(chalk.dim(`  Warmup: ${config.warmup}`));
  console.log(chalk.dim(`  Tools: ${config.tools.join(', ')}`));
  console.log();

  try {
    const results = await runBenchmark(config);
    await generateReport(results, config);
  } catch (error) {
    console.error(chalk.red('\n❌ Benchmark failed:'), error.message);
    process.exit(1);
  }
}

main();

