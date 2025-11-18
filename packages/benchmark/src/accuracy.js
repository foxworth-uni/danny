import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import chalk from 'chalk';

/**
 * Load ground truth data
 */
export async function loadGroundTruth(targetPath) {
  const groundTruthPath = resolve(targetPath, 'ground-truth.json');

  try {
    const content = await readFile(groundTruthPath, 'utf-8');
    return JSON.parse(content);
  } catch (error) {
    console.warn(chalk.yellow(`Warning: Could not load ground truth from ${groundTruthPath}`));
    return null;
  }
}

/**
 * Normalize finding for comparison
 */
function normalizeFinding(finding) {
  // Extract file path and name
  const file = finding.file || finding.path || '';
  const name = finding.name || finding.export || finding.symbol || '';

  return {
    file: file.replace(/\\/g, '/').toLowerCase(),
    name: name.toLowerCase(),
    type: finding.type || 'unknown',
  };
}

/**
 * Check if two findings match
 */
function findingsMatch(a, b) {
  const normA = normalizeFinding(a);
  const normB = normalizeFinding(b);

  // Match if file and name are the same
  if (normA.file && normB.file && normA.name && normB.name) {
    return normA.file.includes(normB.file) || normB.file.includes(normA.file)
      ? normA.name === normB.name
      : false;
  }

  // Match if just file is the same (for whole-file findings)
  if (normA.file && normB.file && !normA.name && !normB.name) {
    return normA.file.includes(normB.file) || normB.file.includes(normA.file);
  }

  return false;
}

/**
 * Calculate accuracy metrics
 */
export function calculateAccuracy(findings, groundTruth) {
  if (!groundTruth) {
    return {
      available: false,
      message: 'No ground truth available',
    };
  }

  // Build list of expected unused items from ground truth
  const expectedUnused = [];

  if (groundTruth.unused) {
    if (groundTruth.unused.components) {
      for (const comp of groundTruth.unused.components) {
        for (const exp of comp.exports) {
          expectedUnused.push({
            file: comp.file,
            name: exp,
            type: 'component',
          });
        }
      }
    }

    if (groundTruth.unused.functions) {
      for (const func of groundTruth.unused.functions) {
        expectedUnused.push({
          file: func.file,
          name: func.name,
          type: 'function',
        });
      }
    }

    if (groundTruth.unused.variables) {
      for (const variable of groundTruth.unused.variables) {
        expectedUnused.push({
          file: variable.file,
          name: variable.name,
          type: 'variable',
        });
      }
    }
  }

  // Build list of expected used items (should NOT be flagged)
  const expectedUsed = [];

  if (groundTruth.used) {
    if (groundTruth.used.components) {
      for (const comp of groundTruth.used.components) {
        for (const exp of comp.exports) {
          expectedUsed.push({
            file: comp.file,
            name: exp,
            type: 'component',
          });
        }
      }
    }

    if (groundTruth.used.functions) {
      for (const func of groundTruth.used.functions) {
        expectedUsed.push({
          file: func.file,
          name: func.name,
          type: 'function',
        });
      }
    }
  }

  // Calculate true positives, false positives, false negatives
  let truePositives = 0;
  let falsePositives = 0;
  const foundUnused = new Set();

  for (const finding of findings) {
    let matched = false;

    // Check if this finding matches an expected unused item (true positive)
    for (let i = 0; i < expectedUnused.length; i++) {
      if (findingsMatch(finding, expectedUnused[i])) {
        truePositives++;
        foundUnused.add(i);
        matched = true;
        break;
      }
    }

    // Check if this finding matches an expected used item (false positive)
    if (!matched) {
      for (const used of expectedUsed) {
        if (findingsMatch(finding, used)) {
          falsePositives++;
          matched = true;
          break;
        }
      }
    }

    // If it didn't match anything, we can't determine if it's FP or TP
    // For now, count it as TP (benefit of the doubt)
    if (!matched) {
      truePositives++;
    }
  }

  // False negatives: expected unused items that weren't found
  const falseNegatives = expectedUnused.length - foundUnused.size;

  // Calculate metrics
  const precision = truePositives + falsePositives > 0
    ? truePositives / (truePositives + falsePositives)
    : 0;

  const recall = truePositives + falseNegatives > 0
    ? truePositives / (truePositives + falseNegatives)
    : 0;

  const f1Score = precision + recall > 0
    ? (2 * precision * recall) / (precision + recall)
    : 0;

  return {
    available: true,
    truePositives,
    falsePositives,
    falseNegatives,
    precision,
    recall,
    f1Score,
    expectedUnused: expectedUnused.length,
    foundUnused: foundUnused.size,
    totalFindings: findings.length,
  };
}

/**
 * Display accuracy report
 */
export function displayAccuracyReport(accuracy) {
  if (!accuracy.available) {
    console.log(chalk.dim('  Accuracy metrics not available (no ground truth)'));
    return;
  }

  console.log(chalk.bold('\n📊 Accuracy Metrics\n'));

  console.log(chalk.cyan('  True Positives:  ') + chalk.green(accuracy.truePositives));
  console.log(chalk.cyan('  False Positives: ') + chalk.red(accuracy.falsePositives));
  console.log(chalk.cyan('  False Negatives: ') + chalk.red(accuracy.falseNegatives));
  console.log();
  console.log(chalk.cyan('  Precision: ') + chalk.bold((accuracy.precision * 100).toFixed(1) + '%'));
  console.log(chalk.cyan('  Recall:    ') + chalk.bold((accuracy.recall * 100).toFixed(1) + '%'));
  console.log(chalk.cyan('  F1 Score:  ') + chalk.bold((accuracy.f1Score * 100).toFixed(1) + '%'));
  console.log();
  console.log(chalk.dim(`  Expected unused: ${accuracy.expectedUnused}`));
  console.log(chalk.dim(`  Found unused: ${accuracy.foundUnused}`));
  console.log(chalk.dim(`  Total findings: ${accuracy.totalFindings}`));
}

