let pdfjsLibPromise = null;

async function getPdfJs() {
  if (!pdfjsLibPromise) {
    pdfjsLibPromise = import('https://cdn.jsdelivr.net/npm/pdfjs-dist@4.9.155/build/pdf.min.mjs')
      .then(pdfjsLib => {
        pdfjsLib.GlobalWorkerOptions.workerSrc = 'https://cdn.jsdelivr.net/npm/pdfjs-dist@4.9.155/build/pdf.worker.min.mjs';
        return pdfjsLib;
      });
  }
  return pdfjsLibPromise;
}

export async function extractPdfData(arrayBuffer) {
  const pdfjsLib = await getPdfJs();
  const doc = await pdfjsLib.getDocument({ data: arrayBuffer }).promise;
  const pages = [];
  for (let i = 1; i <= doc.numPages; i++) {
    const page = await doc.getPage(i);
    const content = await page.getTextContent();
    pages.push(textFromPdfItems(content.items));
  }
  return {
    text: pages.join('\n\n'),
    sections: remapPdfSectionsToBlockIndices(await extractPdfSections(doc, pages), pages),
  };
}

function textFromPdfItems(items) {
  const lines = [];
  for (const item of items || []) {
    const text = (item.str || '').trim();
    if (!text) continue;
    const transform = item.transform || [];
    const x = Number(transform[4] ?? 0);
    const y = Number(transform[5] ?? 0);
    let line = lines.find(candidate => Math.abs(candidate.y - y) <= 2);
    if (!line) {
      line = { y, items: [] };
      lines.push(line);
    }
    line.items.push({ text, x, width: Number(item.width ?? 0) });
  }

  return lines
    .sort((a, b) => b.y - a.y)
    .map(line => {
      let out = '';
      let prevRight = null;
      for (const item of line.items.sort((a, b) => a.x - b.x)) {
        if (prevRight !== null) {
          const gap = item.x - prevRight;
          out += gap > 18 ? '    ' : ' ';
        }
        out += item.text;
        prevRight = item.x + item.width;
      }
      return out.trimEnd();
    })
    .filter(Boolean)
    .join('\n');
}

async function extractPdfSections(doc, pages) {
  const outline = await doc.getOutline();
  if (outline?.length) {
    const sections = [];
    await collectPdfOutlineSections(doc, outline, 1, sections);
    const deduped = dedupePdfSections(sections, pages.length);
    if (deduped.length) return deduped;
  }
  return inferPdfSectionsFromPages(pages);
}

async function collectPdfOutlineSections(doc, items, level, sections) {
  for (const item of items || []) {
    const title = (item.title || '').replace(/\s+/g, ' ').trim();
    const pageIndex = await resolvePdfDestinationPage(doc, item.dest);
    if (title && Number.isInteger(pageIndex) && pageIndex >= 0) {
      sections.push({ title, level, page_index: pageIndex });
    }
    if (item.items?.length) {
      await collectPdfOutlineSections(doc, item.items, level + 1, sections);
    }
  }
}

async function resolvePdfDestinationPage(doc, dest) {
  let resolved = dest;
  if (typeof resolved === 'string') {
    resolved = await doc.getDestination(resolved);
  }
  if (!Array.isArray(resolved) || resolved.length === 0) return null;
  const target = resolved[0];
  if (Number.isInteger(target)) return target;
  if (target && typeof target === 'object' && 'num' in target && 'gen' in target) {
    return doc.getPageIndex(target);
  }
  return null;
}

function dedupePdfSections(sections, pageCount) {
  const seen = new Set();
  return sections.filter(section => {
    if (!Number.isInteger(section.page_index)) return false;
    if (section.page_index < 0 || section.page_index >= pageCount) return false;
    const key = `${section.title}|${section.level}|${section.page_index}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function inferPdfSectionsFromPages(pages) {
  return pages.flatMap((pageText, index) => {
    const title = inferPdfSectionTitle(pageText);
    return title ? [{ title, level: 1, page_index: index }] : [];
  });
}

function inferPdfSectionTitle(pageText) {
  const compact = pageText.replace(/\s+/g, ' ').trim();
  if (!compact) return null;
  const sample = compact.slice(0, 140);
  for (const pattern of [
    /\bchapter\s+\d+\b[^.!?]{0,90}/i,
    /\bpart\s+[ivxlcdm]+\b[^.!?]{0,90}/i,
    /\b\d+\.\s+[A-Z][^.!?]{0,90}/,
  ]) {
    const match = sample.match(pattern);
    if (match) return match[0].trim();
  }
  return null;
}

function sanitizePdfText(text) {
  let out = '';
  let prevSpace = false;
  for (const ch of text) {
    switch (ch) {
      case '\u200B':
      case '\u200C':
      case '\u200D':
      case '\uFEFF':
      case '\u00AD':
      case '\u2060':
      case '\u180E':
      case '\u034F':
      case '\r':
        break;
      case '\n':
        prevSpace = false;
        out += '\n';
        break;
      case '\t':
      case ' ':
        if (!prevSpace) {
          out += ' ';
          prevSpace = true;
        }
        break;
      default:
        if (/[\u0000-\u001F\u007F-\u009F]/.test(ch)) break;
        prevSpace = false;
        out += ch;
        break;
    }
  }
  return out;
}

function countBlocksFromPlainText(text) {
  const cleaned = sanitizePdfText(text);
  let count = 0;
  for (const para of cleaned.split('\n\n')) {
    const flat = para.replace(/\n/g, ' ');
    if (flat.trim()) count += 1;
  }
  return count;
}

function remapPdfSectionsToBlockIndices(sections, pages) {
  let blocksBeforePage = 0;
  const pageBlockStarts = pages.map(pageText => {
    const start = blocksBeforePage;
    blocksBeforePage += countBlocksFromPlainText(pageText);
    return start;
  });
  return sections.map(section => ({
    title: section.title,
    level: section.level,
    block_index: pageBlockStarts[section.page_index] ?? blocksBeforePage,
  }));
}

