export function fallbackExtractArticle() {
  const url = location.href;
  const title = findTitle();
  const root = findArticleRoot();
  const markdown = root ? markdownFromRoot(root, title, url) : '';
  const documentMarkdown = markdown && countWords(markdown) > 20
    ? markdown
    : readableDocumentMarkdown(title, url);
  const fallback = documentMarkdown || plainTextFallback(title, url);

  return {
    markdown: fallback,
    title,
    url,
    byline: findByline(),
    wordCount: countWords(fallback),
    selected: false
  };
}

export function findTitle() {
  return normalizeText(
    document.querySelector('meta[property="og:title"]')?.content
    || document.querySelector('h1')?.textContent
    || document.title
    || 'Web page'
  );
}

export function findByline() {
  return normalizeText(
    document.querySelector('[rel="author"]')?.textContent
    || document.querySelector('[class*="byline" i]')?.textContent
    || document.querySelector('[class*="author" i]')?.textContent
    || ''
  ) || undefined;
}

export function normalizeText(value) {
  return String(value || '').replace(/\s+/g, ' ').trim();
}

export function compactMarkdown(value) {
  return value
    .replace(/[ \t]+\n/g, '\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

export function countWords(value) {
  return normalizeText(value).split(/\s+/).filter(Boolean).length;
}

export function sourceLine(pageUrl) {
  try {
    return `Source: ${new URL(pageUrl).hostname}`;
  } catch {
    return `Source: ${pageUrl}`;
  }
}

function findArticleRoot() {
  const candidates = [
    ...document.querySelectorAll([
      'article',
      'main',
      '[role="main"]',
      '.article',
      '.post',
      '.entry-content',
      '.post-content',
      '.content',
      '.knc-content',
      '.knc-sapo',
      '.detail-content',
      '.news-content',
      '.article-content',
      '[id*="content" i]',
      '[class*="content" i]',
      '[id*="article" i]',
      '[class*="article" i]',
      '[id*="detail" i]',
      '[class*="detail" i]'
    ].join(','))
  ];
  if (!candidates.length && document.body) candidates.push(document.body);

  let best = null;
  let bestScore = 0;
  for (const candidate of candidates) {
    const clone = candidate.cloneNode(true);
    removeNoise(clone);
    const text = normalizeText(clone.textContent || '');
    const paragraphCount = clone.querySelectorAll('p').length;
    const score = text.length + paragraphCount * 120;
    if (text.length > 200 && score > bestScore) {
      best = clone;
      bestScore = score;
    }
  }
  return best;
}

function removeNoise(root) {
  root.querySelectorAll([
    'script',
    'style',
    'noscript',
    'template',
    'svg',
    'canvas',
    'iframe',
    'form',
    'input',
    'button',
    'select',
    'textarea',
    'nav',
    'aside',
    'footer',
    'header',
    '[aria-hidden="true"]',
    '[hidden]',
    '.ad',
    '.ads',
    '.advertisement',
    '.banner',
    '.cookie',
    '.comments',
    '.footer',
    '.header',
    '.menu',
    '.nav',
    '.newsletter',
    '.promo',
    '.related',
    '.share',
    '.sidebar',
    '.social'
  ].join(',')).forEach(node => node.remove());
}

function markdownFromRoot(root, pageTitle, pageUrl) {
  const parts = [`# ${pageTitle}`, '', sourceLine(pageUrl), ''];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT, {
    acceptNode(node) {
      if (!node.textContent || !normalizeText(node.textContent)) {
        return NodeFilter.FILTER_REJECT;
      }
      return NodeFilter.FILTER_ACCEPT;
    }
  });

  const seen = new Set();
  let node = walker.currentNode;
  while (node) {
    appendNode(node, parts, seen);
    node = walker.nextNode();
  }

  return compactMarkdown(parts.join('\n'));
}

function appendNode(node, parts, seen) {
  const tag = node.tagName?.toLowerCase();
  if (!tag || seen.has(node)) return;

  if (/^h[1-6]$/.test(tag)) {
    markSubtree(node, seen);
    const level = Math.min(6, Number(tag.slice(1)) + 1);
    parts.push('', `${'#'.repeat(level)} ${normalizeText(node.textContent)}`, '');
    return;
  }

  if (tag === 'p') {
    markSubtree(node, seen);
    parts.push('', inlineMarkdown(node), '');
    return;
  }

  if (tag === 'blockquote') {
    markSubtree(node, seen);
    const quote = normalizeText(node.textContent);
    if (quote) parts.push('', ...quote.split(/\n+/).map(line => `> ${line}`), '');
    return;
  }

  if (tag === 'pre') {
    markSubtree(node, seen);
    const code = node.textContent?.replace(/\n{3,}/g, '\n\n').trim();
    if (code) parts.push('', '```', code, '```', '');
    return;
  }

  if (tag === 'li') {
    markSubtree(node, seen);
    const text = inlineMarkdown(node);
    if (text) parts.push(`- ${text}`);
  }
}

function markSubtree(node, seen) {
  seen.add(node);
  node.querySelectorAll('*').forEach(child => seen.add(child));
}

function inlineMarkdown(node) {
  const clone = node.cloneNode(true);
  clone.querySelectorAll('script, style, noscript, button, svg').forEach(child => child.remove());
  clone.querySelectorAll('a[href]').forEach(anchor => {
    const label = normalizeText(anchor.textContent || '');
    const href = anchor.getAttribute('href');
    if (!label || !href || href.startsWith('javascript:')) {
      anchor.replaceWith(document.createTextNode(label));
      return;
    }
    const absolute = new URL(href, location.href).href;
    anchor.replaceWith(document.createTextNode(`[${label}](${absolute})`));
  });
  return normalizeText(clone.textContent || '');
}

function plainTextFallback(pageTitle, pageUrl) {
  const body = document.body ? document.body.cloneNode(true) : null;
  if (body) removeNoise(body);
  const text = normalizeText(body?.textContent || document.body?.innerText || '');
  const content = text.length > 160 ? text : 'Tinywins could not extract readable article text from this page.';
  return compactMarkdown(`# ${pageTitle}\n\n${sourceLine(pageUrl)}\n\n${content}`);
}

function readableDocumentMarkdown(pageTitle, pageUrl) {
  const parts = [`# ${pageTitle}`, '', sourceLine(pageUrl), ''];
  const seen = new Set();
  const nodes = [...document.querySelectorAll('h1, h2, h3, p, li, blockquote, figcaption, [data-role="sapo"], [class*="sapo" i]')];

  for (const node of nodes) {
    if (isNoisyNode(node)) continue;
    const text = normalizeText(node.textContent || '');
    if (!isReadableBlock(text)) continue;
    const key = text.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);

    const tag = node.tagName?.toLowerCase();
    if (tag === 'h1') continue;
    if (tag === 'h2' || tag === 'h3') {
      parts.push('', `## ${text}`, '');
    } else if (tag === 'li') {
      parts.push(`- ${text}`);
    } else if (tag === 'blockquote') {
      parts.push('', ...text.split(/\n+/).map(line => `> ${line}`), '');
    } else {
      parts.push('', text, '');
    }
  }

  const markdown = compactMarkdown(parts.join('\n'));
  return countWords(markdown) > countWords(`# ${pageTitle}\n\n${sourceLine(pageUrl)}`) + 10
    ? markdown
    : '';
}

function isReadableBlock(text) {
  if (text.length < 30) return false;
  if (text.length > 6000) return false;
  if (/^(copy link|link bài gốc|lấy link|theo dõi|tin mới|đọc thêm|xem thêm)$/i.test(text)) return false;
  return /[\p{L}\p{N}]/u.test(text);
}

function isNoisyNode(node) {
  return Boolean(node.closest([
    'script',
    'style',
    'noscript',
    'template',
    'nav',
    'aside',
    'footer',
    'header',
    'form',
    '[aria-hidden="true"]',
    '[hidden]',
    '.ad',
    '.ads',
    '.advertisement',
    '.banner',
    '.cookie',
    '.comments',
    '.footer',
    '.header',
    '.menu',
    '.nav',
    '.newsletter',
    '.promo',
    '.related',
    '.share',
    '.sidebar',
    '.social'
  ].join(',')));
}
