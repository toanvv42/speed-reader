#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKG="$ROOT/web/pkg"
DIST="$ROOT/dist"

echo "==> Building WASM with wasm-pack…"
wasm-pack build --target web --release --out-dir "$PKG" "$ROOT"

echo "==> Inlining WASM into single HTML file…"
mkdir -p "$DIST"

# Base64-encode the .wasm binary into a temp file (too large for shell variables on Linux)
WASM_B64_FILE=$(mktemp)
base64 < "$PKG/speed_reader_bg.wasm" | tr -d '\n' > "$WASM_B64_FILE"

# Patch the glue JS: replace the URL constructor with a data: URI
# Use awk instead of sed to avoid "argument list too long" on Linux
PATCHED_JS_FILE=$(mktemp)
awk -v b64file="$WASM_B64_FILE" '
  /new URL\(.*speed_reader_bg\.wasm.*import\.meta\.url\)/ {
    # Read the base64 content from file
    getline b64 < b64file
    close(b64file)
    gsub(/new URL\(.*speed_reader_bg\.wasm.*import\.meta\.url\)/, "\"data:application/wasm;base64," b64 "\"")
  }
  { print }
' "$PKG/speed_reader.js" > "$PATCHED_JS_FILE"

# Build the final single HTML file
{
  # Head + style
  echo '<!DOCTYPE html>'
  echo '<html lang="en">'
  echo '<head>'
  awk '
    /<head>/ { in_head = 1; next }
    /<\/head>/ { in_head = 0 }
    in_head { print }
  ' "$ROOT/web/index.html"
  echo '</head>'

  # Body (everything between <body> and the first <script)
  sed -n '/<body>/,/<script/p' "$ROOT/web/index.html" | sed '/<script/d'

  # Inlined script: patched glue JS + app logic
  echo '<script type="module">'
  echo '// === Inlined wasm-bindgen glue (patched for data: URL) ==='
  cat "$PATCHED_JS_FILE"
  echo ''
  echo 'await __wbg_init();'
  echo '// WebReader is already available from the glue above'
  echo ''
  echo '// === Inlined app modules ==='
  cat "$ROOT/web/src/storage.js"
  echo ''
  cat "$ROOT/web/src/pdf.js"
  echo ''

  # App logic (after END_WASM_INLINE_MARKER to </script>)
  sed -n '/END_WASM_INLINE_MARKER/,/<\/script>/p' "$ROOT/web/index.html" \
    | sed '1d' \
    | sed '/<\/script>/d'

  echo '</script>'
  echo '</body></html>'
} > "$DIST/index.html"

# Cleanup temp files
rm -f "$WASM_B64_FILE" "$PATCHED_JS_FILE"

# Copy social sharing assets
cp "$ROOT/web/og.svg" "$DIST/og.svg"
cp "$ROOT/web/favicon.svg" "$DIST/favicon.svg"

# GitHub Pages custom domain. Without this file in the uploaded
# artifact, each Pages deploy would clear the custom domain.
echo 'tinywins.us' > "$DIST/CNAME"

SIZE=$(wc -c < "$DIST/index.html" | tr -d ' ')
SIZE_KB=$((SIZE / 1024))
echo "==> Done! dist/index.html (${SIZE_KB} KB)"
echo "    Open it with: open $DIST/index.html"
