export function extractArticle() {
  const url = location.href;
  const title = normalizeText(
    document.querySelector('meta[property="og:title"]')?.content
    || document.querySelector('h1')?.textContent
    || document.title
    || 'Web page'
  );

  const root = findArticleRoot();
  const markdown = root ? markdownFromRoot(root, title, url) : '';
  const fallback = markdown || plainTextFallback(title, url);

  return {
    markdown: fallback,
    title,
    url,
    byline: findByline(),
    wordCount: countWords(fallback)
  };

  function findArticleRoot() {
    const candidates = [
      ...document.querySelectorAll('article, main, [role="main"], .article, .post, .entry-content, .post-content, .content')
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
      return;
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

  function sourceLine(pageUrl) {
    try {
      return `Source: ${new URL(pageUrl).hostname}`;
    } catch {
      return `Source: ${pageUrl}`;
    }
  }

  function plainTextFallback(pageTitle, pageUrl) {
    const body = document.body ? document.body.cloneNode(true) : null;
    if (body) removeNoise(body);
    const text = normalizeText(body?.textContent || document.body?.innerText || '');
    const content = text.length > 160 ? text : 'Tinywins could not extract readable article text from this page.';
    return compactMarkdown(`# ${pageTitle}\n\n${sourceLine(pageUrl)}\n\n${content}`);
  }

  function findByline() {
    return normalizeText(
      document.querySelector('[rel="author"]')?.textContent
      || document.querySelector('[class*="byline" i]')?.textContent
      || document.querySelector('[class*="author" i]')?.textContent
      || ''
    ) || undefined;
  }

  function normalizeText(value) {
    return String(value || '').replace(/\s+/g, ' ').trim();
  }

  function compactMarkdown(value) {
    return value
      .replace(/[ \t]+\n/g, '\n')
      .replace(/\n{3,}/g, '\n\n')
      .trim();
  }

  function countWords(value) {
    return normalizeText(value).split(/\s+/).filter(Boolean).length;
  }
}
