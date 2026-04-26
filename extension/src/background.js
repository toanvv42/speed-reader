import { createSessionId, storeReaderSession } from './session.js';

chrome.action.onClicked.addListener(tab => {
  openCurrentTabInReader(tab).catch(error => {
    console.error('Tinywins reader failed:', error);
  });
});

chrome.commands.onCommand.addListener(command => {
  if (command !== 'open_reader') return;
  chrome.tabs.query({ active: true, currentWindow: true }, tabs => {
    openCurrentTabInReader(tabs[0]).catch(error => {
      console.error('Tinywins reader command failed:', error);
    });
  });
});

async function openCurrentTabInReader(tab) {
  if (!tab?.id || !tab.url) return;

  const extracted = await extractFromTab(tab).catch(error => ({
    markdown: fallbackMarkdown(tab.url, error),
    title: 'Could not extract page',
    url: tab.url
  }));

  const sessionId = createSessionId();
  await storeReaderSession(sessionId, {
    markdown: extracted.markdown,
    title: extracted.title,
    url: extracted.url || tab.url,
    selected: false
  });

  const readerUrl = chrome.runtime.getURL(
    `reader.html?sessionId=${encodeURIComponent(sessionId)}&returnUrl=${encodeURIComponent(tab.url)}`
  );
  await chrome.tabs.create({ url: readerUrl, index: typeof tab.index === 'number' ? tab.index + 1 : undefined });
}

async function extractFromTab(tab) {
  await chrome.scripting.executeScript({
    target: { tabId: tab.id },
    files: ['extract.js']
  });
  const [result] = await chrome.scripting.executeScript({
    target: { tabId: tab.id },
    func: () => globalThis.__tinywinsExtractArticle()
  });
  if (!result?.result?.markdown) {
    throw new Error('No readable page content was returned.');
  }
  return result.result;
}

function fallbackMarkdown(url, error) {
  const detail = error?.message ? `\n\nError: ${error.message}` : '';
  return [
    '# Could not extract this page',
    '',
    'Tinywins could not extract readable article text from the active tab.',
    '',
    `Original page: ${url}`,
    detail
  ].join('\n');
}
