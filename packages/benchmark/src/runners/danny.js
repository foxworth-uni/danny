import { exec } from 'node:child_process';
import { promisify } from 'node:util';
import { resolve } from 'node:path';

const execAsync = promisify(exec);

/**
 * Run Danny analyzer standalone
 */
export async function runDanny(target, options = {}) {
  const dannyPath = resolve(process.cwd(), '../../target/release/danny');
  const targetPath = resolve(process.cwd(), target);

  const args = [];
  if (options.format) args.push(`--format ${options.format}`);
  if (options.verbose) args.push('--verbose');

  const command = `${dannyPath} "${targetPath}" ${args.join(' ')}`;

  console.log(`Running: ${command}`);

  try {
    const { stdout, stderr } = await execAsync(command, {
      maxBuffer: 10 * 1024 * 1024,
    });

    return {
      success: true,
      stdout,
      stderr,
    };
  } catch (error) {
    return {
      success: false,
      stdout: error.stdout,
      stderr: error.stderr,
      error: error.message,
    };
  }
}

// Allow running standalone
if (import.meta.url === `file://${process.argv[1]}`) {
  const target = process.argv[2] || '../../test-files/nextjs-app';
  const result = await runDanny(target, { format: 'json' });

  console.log('\n=== Danny Results ===\n');
  console.log(result.stdout || result.stderr);

  if (!result.success) {
    console.error('\nError:', result.error);
    process.exit(1);
  }
}

