import Defuddle from 'defuddle/full';
import {
  compactMarkdown,
  countWords,
  fallbackExtractArticle,
  findByline,
  findTitle,
  normalizeText
} from './extract-fallback.js';

export async function extractArticle() {
  const fallback = fallbackExtractArticle();
  try {
    const article = await extractWithDefuddle();
    if (isBetterExtraction(article, fallback)) return article;
  } catch (error) {
    console.warn('Defuddle extraction failed, falling back:', error);
  }

  return fallback;
}

async function extractWithDefuddle() {
  const url = location.href;
  const result = await withSuppressedDefuddleSchemaErrors(() => (
    new Defuddle(document, {
      markdown: true,
      url,
      useAsync: false
    }).parseAsync()
  ));

  const title = normalizeText(result.title || findTitle());
  const markdown = contentOnlyMarkdown(result.content || '', title);

  return {
    markdown,
    title,
    url,
    byline: normalizeText(result.author || findByline() || '') || undefined,
    wordCount: Number.isFinite(result.wordCount) ? result.wordCount : countWords(markdown),
    selected: false
  };
}

function contentOnlyMarkdown(content, title) {
  let markdown = compactMarkdown(content);
  if (!title) return markdown;

  const escaped = title.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return compactMarkdown(markdown.replace(new RegExp(`^#\\s+${escaped}\\s*\\n+`, 'i'), ''));
}

function isBetterExtraction(candidate, fallback) {
  if (!candidate?.markdown) return false;
  const candidateWords = countWords(candidate.markdown);
  const fallbackWords = countWords(fallback?.markdown || '');
  if (candidateWords < 80 && fallbackWords >= candidateWords) return false;
  if (fallbackWords >= 120 && candidateWords < fallbackWords * 0.7) return false;
  return candidateWords >= 3;
}

async function withSuppressedDefuddleSchemaErrors(callback) {
  const originalError = console.error;
  console.error = (...args) => {
    if (typeof args[0] === 'string' && args[0].startsWith('Defuddle: Error parsing schema.org data:')) {
      return;
    }
    originalError.apply(console, args);
  };
  try {
    return await callback();
  } finally {
    console.error = originalError;
  }
}
