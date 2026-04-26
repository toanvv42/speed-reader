import { cp, mkdir, rm } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { buildReader } from './build-reader.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const extensionRoot = resolve(__dirname, '..');
const dist = join(extensionRoot, 'dist');

await rm(dist, { recursive: true, force: true });
await mkdir(dist, { recursive: true });

await cp(join(extensionRoot, 'manifest.json'), join(dist, 'manifest.json'));
await cp(join(extensionRoot, 'src', 'background.js'), join(dist, 'background.js'));
await cp(join(extensionRoot, 'src', 'extract.js'), join(dist, 'extract.js'));
await cp(join(extensionRoot, 'src', 'session.js'), join(dist, 'session.js'));
await cp(join(extensionRoot, 'assets'), join(dist, 'assets'), { recursive: true });

await buildReader({ extensionRoot, dist });

console.log(`Built extension: ${dist}`);
