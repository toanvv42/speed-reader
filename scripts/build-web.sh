#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKG="$ROOT/web/pkg"
DIST="$ROOT/dist"

echo "==> Building WASM with wasm-pack…"
wasm-pack build --target web --release --out-dir "$PKG" "$ROOT"

echo "==> Inlining WASM into single HTML file…"
mkdir -p "$DIST"

# Base64-encode the .wasm binary
WASM_B64=$(base64 < "$PKG/speed_reader_bg.wasm" | tr -d '\n')

# Read the glue JS
GLUE_JS=$(cat "$PKG/speed_reader.js")

# Patch the glue JS: replace the fetch-based init with a data-URL init
# The default init function fetches from a URL; we replace that URL with a data: URI
PATCHED_JS=$(echo "$GLUE_JS" | sed "s|new URL('speed_reader_bg.wasm', import.meta.url)|'data:application/wasm;base64,${WASM_B64}'|")

# Read the HTML template
HTML=$(cat "$ROOT/web/index.html")

# Build the inlined script block
INLINE_SCRIPT="<script type=\"module\">
${PATCHED_JS}

await __wbg_init();
const { WebReader } = await import('data:text/javascript,export { WebReader } from \"./this-is-unused\";');
"

# Actually, we need a simpler approach: just inline everything into one script
# The WebReader class is already defined in the glue JS, we just need to init + use it
read -r -d '' INLINE_BLOCK << 'HEREDOC_END' || true
// BEGIN INLINED WASM MODULE
HEREDOC_END

# Simpler approach: build the complete self-contained script
cat > "$DIST/index.html" << 'HTML_HEAD'
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>speed-reader</title>
HTML_HEAD

# Extract style from template
sed -n '/<style>/,/<\/style>/p' "$ROOT/web/index.html" >> "$DIST/index.html"

echo '</head>' >> "$DIST/index.html"

# Extract body (without scripts)
sed -n '/<body>/,/<script/p' "$ROOT/web/index.html" | sed '/<script/d' >> "$DIST/index.html"

# Write the inlined script
cat >> "$DIST/index.html" << SCRIPT_START
<script type="module">
// === Inlined wasm-bindgen glue (patched for data: URL) ===
${PATCHED_JS}

await __wbg_init();
// WebReader is already available from the glue above

SCRIPT_START

# Extract the app logic (after WASM_INLINE_MARKER to END_WASM_INLINE_MARKER)
sed -n '/END_WASM_INLINE_MARKER/,/<\/script>/p' "$ROOT/web/index.html" \
  | sed '1d' \
  | sed '/<\/script>/d' \
  >> "$DIST/index.html"

echo '</script>' >> "$DIST/index.html"
echo '</body></html>' >> "$DIST/index.html"

SIZE=$(wc -c < "$DIST/index.html" | tr -d ' ')
SIZE_KB=$((SIZE / 1024))
echo "==> Done! dist/index.html (${SIZE_KB} KB)"
echo "    Open it with: open $DIST/index.html"
