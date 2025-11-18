#!/usr/bin/env node

import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import chalk from 'chalk';
import { table } from 'table';

/**
 * Load all benchmark results from the results directory
 */
async function loadResults(resultsDir = './results') {
  const files = await readdir(resultsDir);
  const benchmarkFiles = files.filter(f => f.startsWith('benchmark-') && f.endsWith('.json'));

  const results = [];
  for (const file of benchmarkFiles) {
    const content = await readFile(join(resultsDir, file), 'utf-8');
    results.push(JSON.parse(content));
  }

  return results.sort((a, b) => new Date(b.timestamp) - new Date(a.timestamp));
}

/**
 * Compare multiple benchmark runs
 */
function compareRuns(runs) {
  if (runs.length === 0) {
    console.log(chalk.yellow('No benchmark results found.'));
    return;
  }

  console.log(chalk.bold.cyan('\n📊 Benchmark History\n'));

  const data = [
    ['Date', 'Tool', 'Avg Time', 'Peak Memory', 'Findings'],
  ];

  for (const run of runs.slice(0, 10)) {
    // Show last 10 runs
    const date = new Date(run.timestamp).toLocaleString();

    for (const [tool, result] of Object.entries(run.results)) {
      if (!result.success) continue;

      data.push([
        chalk.dim(date),
        chalk.cyan(tool),
        `${(result.executionTime.avg / 1000).toFixed(2)}s`,
        `${(result.memory.avg / 1024 / 1024).toFixed(2)} MB`,
        result.findings.avg.toFixed(0),
      ]);
    }
  }

  console.log(table(data));
}

/**
 * Show trends over time
 */
function showTrends(runs) {
  if (runs.length < 2) {
    console.log(chalk.yellow('\nNeed at least 2 runs to show trends.'));
    return;
  }

  console.log(chalk.bold.cyan('\n📈 Performance Trends\n'));

  const tools = new Set();
  for (const run of runs) {
    for (const tool of Object.keys(run.results)) {
      if (run.results[tool].success) {
        tools.add(tool);
      }
    }
  }

  for (const tool of tools) {
    const toolRuns = runs
      .filter(r => r.results[tool]?.success)
      .slice(0, 5); // Last 5 runs

    if (toolRuns.length < 2) continue;

    const latest = toolRuns[0].results[tool];
    const previous = toolRuns[1].results[tool];

    const timeDiff = latest.executionTime.avg - previous.executionTime.avg;
    const timeChange = (timeDiff / previous.executionTime.avg) * 100;

    const memDiff = latest.memory.avg - previous.memory.avg;
    const memChange = (memDiff / previous.memory.avg) * 100;

    console.log(chalk.bold(tool));

    if (Math.abs(timeChange) > 1) {
      const arrow = timeChange > 0 ? '↑' : '↓';
      const color = timeChange > 0 ? chalk.red : chalk.green;
      console.log(
        color(`  ${arrow} Time: ${Math.abs(timeChange).toFixed(1)}% (${(timeDiff / 1000).toFixed(2)}s)`)
      );
    } else {
      console.log(chalk.dim('  ≈ Time: No significant change'));
    }

    if (Math.abs(memChange) > 1) {
      const arrow = memChange > 0 ? '↑' : '↓';
      const color = memChange > 0 ? chalk.red : chalk.green;
      console.log(
        color(`  ${arrow} Memory: ${Math.abs(memChange).toFixed(1)}% (${(memDiff / 1024 / 1024).toFixed(2)} MB)`)
      );
    } else {
      console.log(chalk.dim('  ≈ Memory: No significant change'));
    }

    console.log();
  }
}

/**
 * Main comparison function
 */
async function main() {
  try {
    const runs = await loadResults();

    if (runs.length === 0) {
      console.log(chalk.yellow('\nNo benchmark results found. Run a benchmark first:\n'));
      console.log(chalk.dim('  pnpm benchmark\n'));
      return;
    }

    console.log(chalk.dim(`Found ${runs.length} benchmark run(s)\n`));

    compareRuns(runs);
    showTrends(runs);

    console.log(chalk.dim('\nTo run a new benchmark:'));
    console.log(chalk.dim('  pnpm benchmark\n'));
  } catch (error) {
    console.error(chalk.red('Error loading results:'), error.message);
    process.exit(1);
  }
}

main();

