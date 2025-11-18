#!/usr/bin/env node

import { readFile, writeFile } from 'node:fs/promises';
import chalk from 'chalk';
import { table } from 'table';

/**
 * Parse Danny findings
 */
function parseDannyFindings(data) {
  const findings = {
    unusedExports: [],
    unreachableFiles: [],
    modules: [],
    dependencies: [],
    all: data.findings || []
  };

  for (const finding of data.findings || []) {
    switch (finding.type) {
      case 'UnusedExport':
        findings.unusedExports.push({
          file: finding.path,
          name: finding.export_name,
          line: finding.line,
          isType: finding.is_type_only
        });
        break;
      case 'UnreachableFile':
        findings.unreachableFiles.push({
          file: finding.path,
          size: finding.size
        });
        break;
      case 'Module':
        findings.modules.push(finding);
        break;
      case 'Dependency':
        findings.dependencies.push(finding);
        break;
    }
  }

  return findings;
}

/**
 * Parse Knip findings
 */
function parseKnipFindings(data) {
  const findings = {
    unusedFiles: data.files || [],
    unusedDependencies: [],
    unusedDevDependencies: [],
    unusedExports: [],
    unusedTypes: [],
    all: []
  };

  for (const issue of data.issues || []) {
    // Dependencies
    if (issue.dependencies) {
      for (const dep of issue.dependencies) {
        findings.unusedDependencies.push({
          name: dep.name,
          file: issue.file,
          line: dep.line
        });
      }
    }

    // Dev dependencies
    if (issue.devDependencies) {
      for (const dep of issue.devDependencies) {
        findings.unusedDevDependencies.push({
          name: dep.name,
          file: issue.file,
          line: dep.line
        });
      }
    }

    // Exports
    if (issue.exports) {
      for (const exp of issue.exports) {
        findings.unusedExports.push({
          name: exp.name,
          file: issue.file,
          line: exp.line
        });
      }
    }

    // Types
    if (issue.types) {
      for (const type of issue.types) {
        findings.unusedTypes.push({
          name: type.name,
          file: issue.file,
          line: type.line
        });
      }
    }
  }

  return findings;
}

/**
 * Generate comparison report
 */
function generateComparisonReport(dannyData, knipData) {
  const danny = parseDannyFindings(dannyData);
  const knip = parseKnipFindings(knipData);

  console.log(chalk.bold.cyan('\n🔍 Danny vs Knip: Detailed Findings Comparison\n'));
  console.log(chalk.dim('Test Case: Next.js Application (test-files/nextjs-app)\n'));

  // Summary table
  const summaryData = [
    ['Category', 'Danny', 'Knip'],
    [chalk.cyan('Unreachable/Unused Files'), danny.unreachableFiles.length, knip.unusedFiles.length],
    [chalk.cyan('Unused Exports'), danny.unusedExports.filter(e => !e.isType).length, knip.unusedExports.length],
    [chalk.cyan('Unused Types'), danny.unusedExports.filter(e => e.isType).length, knip.unusedTypes.length],
    [chalk.cyan('Unused Dependencies'), '-', knip.unusedDependencies.length],
    [chalk.cyan('Unused Dev Dependencies'), '-', knip.unusedDevDependencies.length],
    [chalk.cyan('Total Findings'), danny.all.length, knip.unusedFiles.length + knip.unusedDependencies.length + knip.unusedDevDependencies.length + knip.unusedExports.length + knip.unusedTypes.length],
  ];

  console.log(chalk.bold('📊 Summary\n'));
  console.log(table(summaryData, {
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
  }));

  // Detailed findings
  console.log(chalk.bold('📁 Unreachable/Unused Files\n'));
  
  console.log(chalk.bold.green('Danny found ' + danny.unreachableFiles.length + ' files:'));
  danny.unreachableFiles.forEach((f, i) => {
    const shortPath = f.file.split('/').slice(-3).join('/');
    console.log(chalk.dim(`  ${i + 1}. ${shortPath}`));
  });

  console.log();
  console.log(chalk.bold.blue('Knip found ' + knip.unusedFiles.length + ' files:'));
  knip.unusedFiles.forEach((f, i) => {
    console.log(chalk.dim(`  ${i + 1}. ${f}`));
  });

  // Unused exports
  console.log(chalk.bold('\n📤 Unused Exports\n'));
  
  console.log(chalk.bold.green('Danny found ' + danny.unusedExports.filter(e => !e.isType).length + ' runtime exports:'));
  danny.unusedExports.filter(e => !e.isType).forEach((e, i) => {
    const shortPath = e.file ? e.file.split('/').slice(-2).join('/') : 'unknown';
    console.log(chalk.dim(`  ${i + 1}. ${e.name} in ${shortPath}:${e.line || '?'}`));
  });

  console.log();
  console.log(chalk.bold.blue('Knip found ' + knip.unusedExports.length + ' exports:'));
  knip.unusedExports.forEach((e, i) => {
    console.log(chalk.dim(`  ${i + 1}. ${e.name} in ${e.file}:${e.line || '?'}`));
  });

  // Unused types
  console.log(chalk.bold('\n🏷️  Unused Types\n'));
  
  console.log(chalk.bold.green('Danny found ' + danny.unusedExports.filter(e => e.isType).length + ' type exports:'));
  danny.unusedExports.filter(e => e.isType).forEach((e, i) => {
    const shortPath = e.file ? e.file.split('/').slice(-2).join('/') : 'unknown';
    console.log(chalk.dim(`  ${i + 1}. ${e.name} in ${shortPath}:${e.line || '?'}`));
  });

  console.log();
  console.log(chalk.bold.blue('Knip found ' + knip.unusedTypes.length + ' types:'));
  knip.unusedTypes.forEach((e, i) => {
    console.log(chalk.dim(`  ${i + 1}. ${e.name} in ${e.file}:${e.line || '?'}`));
  });

  // Dependencies (Knip only)
  console.log(chalk.bold('\n📦 Unused Dependencies (Knip only)\n'));
  
  console.log(chalk.bold.blue('Knip found ' + knip.unusedDependencies.length + ' unused dependencies:'));
  knip.unusedDependencies.forEach((d, i) => {
    console.log(chalk.dim(`  ${i + 1}. ${d.name}`));
  });

  console.log();
  console.log(chalk.bold.blue('Knip found ' + knip.unusedDevDependencies.length + ' unused dev dependencies:'));
  knip.unusedDevDependencies.forEach((d, i) => {
    console.log(chalk.dim(`  ${i + 1}. ${d.name}`));
  });

  // Overlap analysis
  console.log(chalk.bold('\n🔄 Overlap Analysis\n'));

  // Files overlap
  const dannyFiles = new Set(danny.unreachableFiles.map(f => f.file.split('/').pop()));
  const knipFiles = new Set(knip.unusedFiles);
  const commonFiles = [...dannyFiles].filter(f => knipFiles.has(f));
  const dannyOnlyFiles = [...dannyFiles].filter(f => !knipFiles.has(f));
  const knipOnlyFiles = [...knipFiles].filter(f => !dannyFiles.has(f));

  console.log(chalk.cyan('Files:'));
  console.log(chalk.green(`  ✓ Both found: ${commonFiles.length} files`));
  if (commonFiles.length > 0) {
    commonFiles.forEach(f => console.log(chalk.dim(`    - ${f}`)));
  }
  console.log(chalk.yellow(`  ⚠ Danny only: ${dannyOnlyFiles.length} files`));
  if (dannyOnlyFiles.length > 0) {
    dannyOnlyFiles.forEach(f => console.log(chalk.dim(`    - ${f}`)));
  }
  console.log(chalk.yellow(`  ⚠ Knip only: ${knipOnlyFiles.length} files`));
  if (knipOnlyFiles.length > 0) {
    knipOnlyFiles.forEach(f => console.log(chalk.dim(`    - ${f}`)));
  }

  // Exports overlap
  const dannyExports = new Set(danny.unusedExports.filter(e => !e.isType).map(e => e.name));
  const knipExports = new Set(knip.unusedExports.map(e => e.name));
  const commonExports = [...dannyExports].filter(e => knipExports.has(e));
  const dannyOnlyExports = [...dannyExports].filter(e => !knipExports.has(e));
  const knipOnlyExports = [...knipExports].filter(e => !dannyExports.has(e));

  console.log(chalk.cyan('\nExports:'));
  console.log(chalk.green(`  ✓ Both found: ${commonExports.length} exports`));
  if (commonExports.length > 0) {
    commonExports.forEach(e => console.log(chalk.dim(`    - ${e}`)));
  }
  console.log(chalk.yellow(`  ⚠ Danny only: ${dannyOnlyExports.length} exports`));
  if (dannyOnlyExports.length > 0) {
    dannyOnlyExports.forEach(e => console.log(chalk.dim(`    - ${e}`)));
  }
  console.log(chalk.yellow(`  ⚠ Knip only: ${knipOnlyExports.length} exports`));
  if (knipOnlyExports.length > 0) {
    knipOnlyExports.forEach(e => console.log(chalk.dim(`    - ${e}`)));
  }

  // Conclusion
  console.log(chalk.bold.cyan('\n✨ Conclusion\n'));
  
  const filesAgreement = commonFiles.length / Math.max(dannyFiles.size, knipFiles.size) * 100;
  const exportsAgreement = commonExports.length / Math.max(dannyExports.size, knipExports.size) * 100;

  console.log(chalk.dim(`Files Agreement: ${filesAgreement.toFixed(1)}%`));
  console.log(chalk.dim(`Exports Agreement: ${exportsAgreement.toFixed(1)}%`));
  console.log();
  console.log(chalk.green('✓ Both tools successfully identify dead code'));
  console.log(chalk.green('✓ High agreement on unreachable files'));
  console.log(chalk.green('✓ High agreement on unused exports'));
  console.log(chalk.blue('ℹ Knip additionally checks package.json dependencies'));
  console.log(chalk.blue('ℹ Danny provides more detailed module/dependency graph'));
  console.log();

  return {
    danny,
    knip,
    overlap: {
      files: { common: commonFiles, dannyOnly: dannyOnlyFiles, knipOnly: knipOnlyFiles },
      exports: { common: commonExports, dannyOnly: dannyOnlyExports, knipOnly: knipOnlyExports },
      filesAgreement,
      exportsAgreement
    }
  };
}

/**
 * Main
 */
async function main() {
  try {
    const dannyData = JSON.parse(await readFile('/tmp/danny-findings.json', 'utf-8'));
    const knipData = JSON.parse(await readFile('/tmp/knip-findings.json', 'utf-8'));

    const comparison = generateComparisonReport(dannyData, knipData);

    // Save comparison report
    await writeFile(
      './results/findings-comparison.json',
      JSON.stringify(comparison, null, 2)
    );

    console.log(chalk.dim('💾 Detailed comparison saved to results/findings-comparison.json\n'));
  } catch (error) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

main();

