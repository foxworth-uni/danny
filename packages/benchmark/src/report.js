import { table } from 'table';
import chalk from 'chalk';
import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';

/**
 * Format bytes to human readable string
 */
function formatBytes(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
}

/**
 * Format milliseconds to human readable string
 */
function formatTime(ms) {
  if (ms < 1000) return `${ms.toFixed(2)}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(2)}s`;
  return `${(ms / 60000).toFixed(2)}m`;
}

/**
 * Generate comparison table
 */
function generateComparisonTable(results) {
  const tools = Object.keys(results);

  if (tools.length === 0) {
    return 'No results to display';
  }

  const data = [
    ['Metric', ...tools.map(t => chalk.bold(t))],
  ];

  // Execution time
  data.push([
    chalk.cyan('Avg Time'),
    ...tools.map(t => {
      if (!results[t].success) return chalk.red('Failed');
      return formatTime(results[t].executionTime.avg);
    }),
  ]);

  data.push([
    chalk.cyan('Min Time'),
    ...tools.map(t => {
      if (!results[t].success) return chalk.red('Failed');
      return formatTime(results[t].executionTime.min);
    }),
  ]);

  data.push([
    chalk.cyan('Max Time'),
    ...tools.map(t => {
      if (!results[t].success) return chalk.red('Failed');
      return formatTime(results[t].executionTime.max);
    }),
  ]);

  data.push([
    chalk.cyan('Std Dev'),
    ...tools.map(t => {
      if (!results[t].success) return chalk.red('Failed');
      return formatTime(results[t].executionTime.stddev);
    }),
  ]);

  // Memory
  data.push([
    chalk.cyan('Peak Memory'),
    ...tools.map(t => {
      if (!results[t].success) return chalk.red('Failed');
      return formatBytes(results[t].memory.max);
    }),
  ]);

  data.push([
    chalk.cyan('Avg Memory'),
    ...tools.map(t => {
      if (!results[t].success) return chalk.red('Failed');
      return formatBytes(results[t].memory.avg);
    }),
  ]);

  // Findings
  data.push([
    chalk.cyan('Findings'),
    ...tools.map(t => {
      if (!results[t].success) return chalk.red('Failed');
      return results[t].findings.avg.toFixed(0);
    }),
  ]);

  data.push([
    chalk.cyan('Success Rate'),
    ...tools.map(t => {
      if (!results[t].success) return chalk.red('0%');
      const rate = (results[t].successfulIterations / results[t].iterations) * 100;
      return `${rate.toFixed(0)}%`;
    }),
  ]);

  return table(data, {
    border: {
      topBody: '─',
      topJoin: '┬',
      topLeft: '┌',
      topRight: '┐',
      bottomBody: '─',
      bottomJoin: '┴',
      bottomLeft: '└',
      bottomRight: '┘',
      bodyLeft: '│',
      bodyRight: '│',
      bodyJoin: '│',
      joinBody: '─',
      joinLeft: '├',
      joinRight: '┤',
      joinJoin: '┼',
    },
  });
}

/**
 * Generate detailed findings comparison
 */
function generateFindingsComparison(results) {
  const tools = Object.keys(results);
  const output = [];

  output.push(chalk.bold.cyan('\n📋 Findings Comparison\n'));

  for (const tool of tools) {
    if (!results[tool].success) {
      output.push(chalk.red(`${tool}: Failed - ${results[tool].error}`));
      continue;
    }

    const findings = results[tool].findings.details;
    output.push(chalk.bold(`\n${tool} (${findings.length} findings):`));

    // Group by type
    const byType = {};
    for (const finding of findings) {
      const type = finding.type || 'unknown';
      if (!byType[type]) byType[type] = [];
      byType[type].push(finding);
    }

    for (const [type, items] of Object.entries(byType)) {
      output.push(chalk.dim(`  ${type}: ${items.length}`));
    }
  }

  return output.join('\n');
}

/**
 * Generate performance comparison
 */
function generatePerformanceComparison(results) {
  const tools = Object.keys(results).filter(t => results[t].success);

  if (tools.length < 2) {
    return '\n(Need at least 2 successful results to compare)';
  }

  const output = [];
  output.push(chalk.bold.cyan('\n⚡ Performance Comparison\n'));

  // Find fastest
  const fastest = tools.reduce((a, b) =>
    results[a].executionTime.avg < results[b].executionTime.avg ? a : b
  );

  output.push(chalk.green(`🏆 Fastest: ${fastest} (${formatTime(results[fastest].executionTime.avg)})`));

  // Compare others to fastest
  for (const tool of tools) {
    if (tool === fastest) continue;

    const ratio = results[tool].executionTime.avg / results[fastest].executionTime.avg;
    const slower = ((ratio - 1) * 100).toFixed(1);

    output.push(
      chalk.dim(`   ${tool}: ${formatTime(results[tool].executionTime.avg)} (${slower}% slower)`)
    );
  }

  // Memory comparison
  const leastMemory = tools.reduce((a, b) =>
    results[a].memory.avg < results[b].memory.avg ? a : b
  );

  output.push(chalk.green(`\n💾 Least Memory: ${leastMemory} (${formatBytes(results[leastMemory].memory.avg)})`));

  for (const tool of tools) {
    if (tool === leastMemory) continue;

    const diff = results[tool].memory.avg - results[leastMemory].memory.avg;
    output.push(
      chalk.dim(`   ${tool}: ${formatBytes(results[tool].memory.avg)} (+${formatBytes(diff)})`)
    );
  }

  return output.join('\n');
}

/**
 * Generate HTML report
 */
async function generateHTMLReport(results, config, outputPath) {
  const tools = Object.keys(results);

  const html = `
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Danny Benchmark Report</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: #0f172a;
      color: #e2e8f0;
      padding: 2rem;
      line-height: 1.6;
    }
    .container { max-width: 1200px; margin: 0 auto; }
    h1 { font-size: 2.5rem; margin-bottom: 1rem; color: #38bdf8; }
    h2 { font-size: 1.75rem; margin: 2rem 0 1rem; color: #7dd3fc; }
    .meta { color: #94a3b8; margin-bottom: 2rem; }
    .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 1.5rem; margin: 2rem 0; }
    .card {
      background: #1e293b;
      border: 1px solid #334155;
      border-radius: 0.5rem;
      padding: 1.5rem;
    }
    .card h3 { color: #38bdf8; margin-bottom: 1rem; }
    .metric { display: flex; justify-content: space-between; padding: 0.5rem 0; border-bottom: 1px solid #334155; }
    .metric:last-child { border-bottom: none; }
    .metric-label { color: #94a3b8; }
    .metric-value { font-weight: 600; color: #e2e8f0; }
    .success { color: #22c55e; }
    .warning { color: #eab308; }
    .error { color: #ef4444; }
    table { width: 100%; border-collapse: collapse; margin: 1rem 0; }
    th, td { padding: 0.75rem; text-align: left; border-bottom: 1px solid #334155; }
    th { background: #1e293b; color: #38bdf8; font-weight: 600; }
    tr:hover { background: #1e293b; }
    .winner { background: #064e3b; }
    .findings { background: #1e293b; padding: 1rem; border-radius: 0.5rem; margin: 1rem 0; }
    .findings-list { list-style: none; }
    .findings-list li { padding: 0.25rem 0; color: #94a3b8; }
  </style>
</head>
<body>
  <div class="container">
    <h1>🚀 Danny Benchmark Report</h1>
    <div class="meta">
      <div>Generated: ${new Date().toLocaleString()}</div>
      <div>Target: ${config.target}</div>
      <div>Iterations: ${config.iterations}</div>
    </div>

    <h2>Performance Overview</h2>
    <div class="grid">
      ${tools
        .map(
          tool => `
        <div class="card">
          <h3>${tool}</h3>
          ${
            results[tool].success
              ? `
          <div class="metric">
            <span class="metric-label">Avg Time</span>
            <span class="metric-value">${formatTime(results[tool].executionTime.avg)}</span>
          </div>
          <div class="metric">
            <span class="metric-label">Min Time</span>
            <span class="metric-value">${formatTime(results[tool].executionTime.min)}</span>
          </div>
          <div class="metric">
            <span class="metric-label">Max Time</span>
            <span class="metric-value">${formatTime(results[tool].executionTime.max)}</span>
          </div>
          <div class="metric">
            <span class="metric-label">Peak Memory</span>
            <span class="metric-value">${formatBytes(results[tool].memory.max)}</span>
          </div>
          <div class="metric">
            <span class="metric-label">Findings</span>
            <span class="metric-value">${results[tool].findings.avg.toFixed(0)}</span>
          </div>
          <div class="metric">
            <span class="metric-label">Success Rate</span>
            <span class="metric-value success">${((results[tool].successfulIterations / results[tool].iterations) * 100).toFixed(0)}%</span>
          </div>
          `
              : `<div class="error">Failed: ${results[tool].error}</div>`
          }
        </div>
      `
        )
        .join('')}
    </div>

    <h2>Detailed Comparison</h2>
    <table>
      <thead>
        <tr>
          <th>Metric</th>
          ${tools.map(t => `<th>${t}</th>`).join('')}
        </tr>
      </thead>
      <tbody>
        <tr>
          <td>Avg Execution Time</td>
          ${tools
            .map(t =>
              results[t].success
                ? `<td>${formatTime(results[t].executionTime.avg)}</td>`
                : `<td class="error">Failed</td>`
            )
            .join('')}
        </tr>
        <tr>
          <td>Peak Memory</td>
          ${tools
            .map(t =>
              results[t].success
                ? `<td>${formatBytes(results[t].memory.max)}</td>`
                : `<td class="error">Failed</td>`
            )
            .join('')}
        </tr>
        <tr>
          <td>Findings Count</td>
          ${tools
            .map(t =>
              results[t].success
                ? `<td>${results[t].findings.avg.toFixed(0)}</td>`
                : `<td class="error">Failed</td>`
            )
            .join('')}
        </tr>
      </tbody>
    </table>

    <h2>Findings Details</h2>
    ${tools
      .map(
        tool => `
      <div class="findings">
        <h3>${tool}</h3>
        ${
          results[tool].success
            ? `
        <ul class="findings-list">
          ${results[tool].findings.details
            .slice(0, 10)
            .map(f => `<li>${f.type || 'unknown'}: ${f.file || f.message || JSON.stringify(f)}</li>`)
            .join('')}
          ${results[tool].findings.details.length > 10 ? `<li>... and ${results[tool].findings.details.length - 10} more</li>` : ''}
        </ul>
        `
            : `<div class="error">Failed: ${results[tool].error}</div>`
        }
      </div>
    `
      )
      .join('')}
  </div>
</body>
</html>
  `;

  await writeFile(outputPath, html);
}

/**
 * Generate and display report
 */
export async function generateReport(results, config) {
  console.log(chalk.bold.cyan('\n📊 Benchmark Results\n'));

  // Table comparison
  console.log(generateComparisonTable(results));

  // Performance comparison
  console.log(generatePerformanceComparison(results));

  // Findings comparison
  console.log(generateFindingsComparison(results));

  // Generate HTML report if requested
  if (config.format === 'html') {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const htmlPath = join(config.output, `report-${timestamp}.html`);
    await generateHTMLReport(results, config, htmlPath);
    console.log(chalk.dim(`\n📄 HTML report saved to ${htmlPath}`));
  }

  // Summary
  console.log(chalk.bold.cyan('\n✨ Summary\n'));

  const successful = Object.keys(results).filter(t => results[t].success);

  if (successful.length === 0) {
    console.log(chalk.red('All benchmarks failed!'));
    return;
  }

  const fastest = successful.reduce((a, b) =>
    results[a].executionTime.avg < results[b].executionTime.avg ? a : b
  );

  console.log(chalk.green(`🏆 Winner: ${fastest}`));
  console.log(chalk.dim(`   Time: ${formatTime(results[fastest].executionTime.avg)}`));
  console.log(chalk.dim(`   Memory: ${formatBytes(results[fastest].memory.avg)}`));
  console.log(chalk.dim(`   Findings: ${results[fastest].findings.avg.toFixed(0)}`));
  console.log();
}

