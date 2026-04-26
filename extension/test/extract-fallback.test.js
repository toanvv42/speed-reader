import assert from 'node:assert/strict';
import test from 'node:test';
import { compactMarkdown, countWords } from '../src/extract-fallback.js';

test('compactMarkdown removes extra vertical whitespace', () => {
  assert.equal(compactMarkdown('Title\n\n\n\nBody  \n\n'), 'Title\n\nBody');
});

test('countWords ignores repeated whitespace', () => {
  assert.equal(countWords(' one   two\nthree '), 3);
});
