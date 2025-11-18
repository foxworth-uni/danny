import { exec } from 'node:child_process';
import { promisify } from 'node:util';
import { resolve } from 'node:path';

const execAsync = promisify(exec);

/**
 * Run Knip analyzer standalone
 */
export async function runKnip(target, options = {}) {
  const targetPath = resolve(process.cwd(), target);

  const args = [];
  if (options.reporter) args.push(`--reporter ${options.reporter}`);
  if (options.verbose) args.push('--debug');

  const command = `npx knip ${args.join(' ')}`;

  console.log(`Running: ${command}`);
  console.log(`In directory: ${targetPath}`);

  try {
    const { stdout, stderr } = await execAsync(command, {
      cwd: targetPath,
      maxBuffer: 10 * 1024 * 1024,
    });

    return {
      success: true,
      stdout,
      stderr,
    };
  } catch (error) {
    // Knip exits with non-zero when it finds issues
    return {
      success: error.code === 1, // Code 1 means issues found, which is "success" for us
      stdout: error.stdout,
      stderr: error.stderr,
      error: error.code !== 1 ? error.message : null,
    };
  }
}

// Allow running standalone
if (import.meta.url === `file://${process.argv[1]}`) {
  const target = process.argv[2] || '../../test-files/nextjs-app';
  const result = await runKnip(target, { reporter: 'json' });

  console.log('\n=== Knip Results ===\n');
  console.log(result.stdout || result.stderr);

  if (!result.success && result.error) {
    console.error('\nError:', result.error);
    process.exit(1);
  }
}

