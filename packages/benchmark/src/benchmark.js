import { performance } from 'node:perf_hooks';
import { exec } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import chalk from 'chalk';
import ora from 'ora';

const execAsync = promisify(exec);

/**
 * Measure system resources during execution
 */
class ResourceMonitor {
  constructor() {
    this.samples = [];
    this.interval = null;
  }

  start() {
    this.samples = [];
    this.interval = setInterval(() => {
      const usage = process.memoryUsage();
      this.samples.push({
        timestamp: Date.now(),
        heapUsed: usage.heapUsed,
        heapTotal: usage.heapTotal,
        external: usage.external,
        rss: usage.rss,
      });
    }, 100);
  }

  stop() {
    if (this.interval) {
      clearInterval(this.interval);
      this.interval = null;
    }
  }

  getStats() {
    if (this.samples.length === 0) {
      return {
        peakMemory: 0,
        avgMemory: 0,
        samples: 0,
      };
    }

    const memories = this.samples.map(s => s.rss);
    return {
      peakMemory: Math.max(...memories),
      avgMemory: memories.reduce((a, b) => a + b, 0) / memories.length,
      samples: this.samples.length,
    };
  }
}

/**
 * Run a single benchmark iteration
 */
async function runIteration(tool, target, config) {
  const monitor = new ResourceMonitor();
  const startTime = performance.now();

  monitor.start();

  try {
    let result;
    if (tool === 'danny') {
      result = await runDanny(target);
    } else if (tool === 'knip') {
      result = await runKnip(target);
    } else {
      throw new Error(`Unknown tool: ${tool}`);
    }

    const endTime = performance.now();
    monitor.stop();

    const resourceStats = monitor.getStats();

    return {
      success: true,
      executionTime: endTime - startTime,
      memory: resourceStats,
      findings: result.findings,
      output: result.output,
      error: null,
    };
  } catch (error) {
    const endTime = performance.now();
    monitor.stop();

    return {
      success: false,
      executionTime: endTime - startTime,
      memory: monitor.getStats(),
      findings: [],
      output: '',
      error: error.message,
    };
  }
}

/**
 * Run Danny analyzer
 */
async function runDanny(target) {
  const dannyPath = resolve(process.cwd(), '../../target/release/danny');
  const targetPath = resolve(process.cwd(), target);

  try {
    const { stdout, stderr } = await execAsync(`${dannyPath} "${targetPath}" --format json`, {
      maxBuffer: 10 * 1024 * 1024, // 10MB buffer
    });

    const output = stdout || stderr;
    const findings = parseDannyOutput(output);

    return {
      findings,
      output,
    };
  } catch (error) {
    // Danny might exit with non-zero if it finds issues
    if (error.stdout || error.stderr) {
      const output = error.stdout || error.stderr;
      const findings = parseDannyOutput(output);
      return { findings, output };
    }
    throw error;
  }
}

/**
 * Run Knip analyzer
 */
async function runKnip(target) {
  const targetPath = resolve(process.cwd(), target);

  try {
    const { stdout, stderr } = await execAsync(`npx knip --directory "${targetPath}" --reporter json`, {
      maxBuffer: 10 * 1024 * 1024, // 10MB buffer
      cwd: targetPath,
    });

    let findings = [];
    try {
      const output = stdout || stderr;
      const knipResult = JSON.parse(output);
      findings = parseKnipOutput(knipResult);
    } catch {
      findings = parseKnipTextOutput(stdout || stderr);
    }

    return {
      findings,
      output: stdout || stderr,
    };
  } catch (error) {
    if (error.stdout || error.stderr) {
      try {
        const output = error.stdout || error.stderr;
        const knipResult = JSON.parse(output);
        const findings = parseKnipOutput(knipResult);
        return { findings, output };
      } catch {
        const findings = parseKnipTextOutput(error.stdout || error.stderr);
        return { findings, output: error.stdout || error.stderr };
      }
    }
    throw error;
  }
}

/**
 * Parse Danny JSON output
 */
function parseDannyOutput(output) {
  try {
    const result = JSON.parse(output);
    // Danny outputs an AnalysisResult with a findings array
    return result.findings || [];
  } catch (error) {
    // Fallback to text parsing
    return parseTextOutput(output);
  }
}

/**
 * Parse text output into structured findings
 */
function parseTextOutput(output) {
  const findings = [];
  const lines = output.split('\n');

  for (const line of lines) {
    if (line.includes('unused') || line.includes('dead')) {
      findings.push({
        type: 'unused',
        message: line.trim(),
      });
    }
  }

  return findings;
}

/**
 * Parse Knip JSON output
 */
function parseKnipOutput(knipResult) {
  const findings = [];

  if (knipResult.files) {
    for (const [file, issues] of Object.entries(knipResult.files)) {
      if (issues.exports) {
        for (const exp of issues.exports) {
          findings.push({
            type: 'unused-export',
            file,
            name: exp.name || exp,
            line: exp.line,
          });
        }
      }
      if (issues.dependencies) {
        for (const dep of issues.dependencies) {
          findings.push({
            type: 'unused-dependency',
            file,
            name: dep.name || dep,
          });
        }
      }
    }
  }

  return findings;
}

/**
 * Parse Knip text output
 */
function parseKnipTextOutput(output) {
  const findings = [];
  const lines = output.split('\n');

  for (const line of lines) {
    if (line.includes('unused') || line.includes('Unused')) {
      findings.push({
        type: 'unused',
        message: line.trim(),
      });
    }
  }

  return findings;
}

/**
 * Run multiple iterations and collect statistics
 */
async function runMultipleIterations(tool, target, iterations, warmup) {
  const spinner = ora(`Running ${tool} (warmup: ${warmup}, iterations: ${iterations})`).start();

  const results = [];

  // Warmup runs
  for (let i = 0; i < warmup; i++) {
    spinner.text = `${tool}: Warmup ${i + 1}/${warmup}`;
    await runIteration(tool, target, {});
  }

  // Actual benchmark runs
  for (let i = 0; i < iterations; i++) {
    spinner.text = `${tool}: Iteration ${i + 1}/${iterations}`;
    const result = await runIteration(tool, target, {});
    results.push(result);
  }

  spinner.succeed(`${tool}: Completed ${iterations} iterations`);

  return results;
}

/**
 * Calculate statistics from multiple runs
 */
function calculateStats(results) {
  const successful = results.filter(r => r.success);

  if (successful.length === 0) {
    return {
      success: false,
      error: results[0]?.error || 'All iterations failed',
    };
  }

  const times = successful.map(r => r.executionTime);
  const memories = successful.map(r => r.memory.peakMemory);
  
  // Handle findings properly - it should be an array
  const findingCounts = successful.map(r => {
    const findings = r.findings;
    if (Array.isArray(findings)) {
      return findings.length;
    }
    return 0;
  });

  return {
    success: true,
    iterations: results.length,
    successfulIterations: successful.length,
    executionTime: {
      min: Math.min(...times),
      max: Math.max(...times),
      avg: times.reduce((a, b) => a + b, 0) / times.length,
      median: times.sort((a, b) => a - b)[Math.floor(times.length / 2)],
      stddev: calculateStdDev(times),
    },
    memory: {
      min: Math.min(...memories),
      max: Math.max(...memories),
      avg: memories.reduce((a, b) => a + b, 0) / memories.length,
    },
    findings: {
      min: Math.min(...findingCounts),
      max: Math.max(...findingCounts),
      avg: findingCounts.reduce((a, b) => a + b, 0) / findingCounts.length,
      details: Array.isArray(successful[0].findings) ? successful[0].findings : [], // Use first successful run for details
    },
    rawResults: results,
  };
}

/**
 * Calculate standard deviation
 */
function calculateStdDev(values) {
  const avg = values.reduce((a, b) => a + b, 0) / values.length;
  const squareDiffs = values.map(value => Math.pow(value - avg, 2));
  const avgSquareDiff = squareDiffs.reduce((a, b) => a + b, 0) / squareDiffs.length;
  return Math.sqrt(avgSquareDiff);
}

/**
 * Main benchmark runner
 */
export async function runBenchmark(config) {
  console.log(chalk.bold('\n📊 Running Benchmarks\n'));

  const results = {};

  for (const tool of config.tools) {
    console.log(chalk.cyan(`\n▶ Benchmarking ${tool}...`));

    const iterationResults = await runMultipleIterations(
      tool,
      config.target,
      config.iterations,
      config.warmup
    );

    results[tool] = calculateStats(iterationResults);
  }

  // Save results
  await mkdir(config.output, { recursive: true });
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  const resultPath = join(config.output, `benchmark-${timestamp}.json`);

  await writeFile(
    resultPath,
    JSON.stringify(
      {
        timestamp: new Date().toISOString(),
        config,
        results,
      },
      null,
      2
    )
  );

  console.log(chalk.dim(`\n💾 Results saved to ${resultPath}\n`));

  return results;
}

