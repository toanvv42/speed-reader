import { mkdir } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import packageJson from '../package.json' with { type: 'json' };

const __dirname = dirname(fileURLToPath(import.meta.url));
const extensionRoot = resolve(__dirname, '..');
const outDir = join(extensionRoot, 'packages');
const zipName = `tinywins-speed-reader-${packageJson.version}.zip`;
const zipPath = join(outDir, zipName);

await mkdir(outDir, { recursive: true });

const build = spawnSync(process.execPath, ['scripts/build.mjs'], {
  cwd: extensionRoot,
  stdio: 'inherit'
});
if (build.status !== 0) process.exit(build.status || 1);

const zip = spawnSync('zip', ['-qr', zipPath, '.'], {
  cwd: join(extensionRoot, 'dist'),
  stdio: 'inherit'
});
if (zip.error) {
  console.error(`Could not run zip: ${zip.error.message}`);
  process.exit(1);
}
if (zip.status !== 0) process.exit(zip.status || 1);

console.log(`Packaged extension: ${zipPath}`);
