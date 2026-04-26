import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const defaultExtensionRoot = resolve(__dirname, '..');
const repoRoot = resolve(defaultExtensionRoot, '..');

export async function buildReader({ extensionRoot = defaultExtensionRoot, dist = join(extensionRoot, 'dist') } = {}) {
  const webRoot = join(repoRoot, 'web');
  const pkgRoot = join(webRoot, 'pkg');
  const readerHtmlPath = join(webRoot, 'index.html');

  let html = await readFile(readerHtmlPath, 'utf8');
  html = html
    .split('\n')
    .filter(line => !line.includes("fonts.googleapis.com"))
    .filter(line => !line.includes('property="og:url"'))
    .filter(line => !line.includes('property="og:image"'))
    .filter(line => !line.includes('name="twitter:url"'))
    .filter(line => !line.includes('name="twitter:image"'))
    .join('\n')
    .replace(/<link rel="icon" type="image\/svg\+xml" href="favicon\.svg">/g, '<link rel="icon" type="image/svg+xml" href="assets/icons/icon.svg">');

  const scriptMatch = html.match(/<script type="module">([\s\S]*?)<\/script>\s*<\/body>/);
  if (!scriptMatch) {
    throw new Error('Could not find reader module script in web/index.html');
  }
  const readerScript = scriptMatch[1].trimStart();
  html = html.replace(
    /<script type="module">[\s\S]*?<\/script>\s*<\/body>/,
    '<script type="module" src="reader.js"></script>\n</body>'
  );

  await mkdir(join(dist, 'pkg'), { recursive: true });
  await mkdir(join(dist, 'src'), { recursive: true });
  await writeFile(join(dist, 'reader.html'), html);
  await writeFile(join(dist, 'reader.js'), readerScript);

  await cp(join(pkgRoot, 'speed_reader.js'), join(dist, 'pkg', 'speed_reader.js'));
  await cp(join(pkgRoot, 'speed_reader_bg.wasm'), join(dist, 'pkg', 'speed_reader_bg.wasm'));
  await cp(join(repoRoot, 'web', 'src', 'storage.js'), join(dist, 'src', 'storage.js'));
  await cp(join(extensionRoot, 'src', 'pdf.js'), join(dist, 'src', 'pdf.js'));
}

if (import.meta.url === `file://${process.argv[1]}`) {
  await buildReader();
  console.log(`Built reader: ${join(defaultExtensionRoot, 'dist', 'reader.html')}`);
}
