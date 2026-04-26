import { cp, mkdir, rm } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import { buildReader } from './build-reader.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const extensionRoot = resolve(__dirname, '..');
const dist = join(extensionRoot, 'dist');

await rm(dist, { recursive: true, force: true });
await mkdir(dist, { recursive: true });

await cp(join(extensionRoot, 'manifest.json'), join(dist, 'manifest.json'));
await cp(join(extensionRoot, 'src', 'background.js'), join(dist, 'background.js'));
await cp(join(extensionRoot, 'src', 'session.js'), join(dist, 'session.js'));
await cp(join(extensionRoot, 'assets'), join(dist, 'assets'), { recursive: true });

await build({
  bundle: true,
  entryPoints: [join(extensionRoot, 'src', 'extract-entry.js')],
  format: 'iife',
  outfile: join(dist, 'extract.js'),
  platform: 'browser',
  target: 'chrome120'
});

await buildReader({ extensionRoot, dist });

console.log(`Built extension: ${dist}`);
