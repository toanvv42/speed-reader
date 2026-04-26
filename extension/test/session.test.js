import assert from 'node:assert/strict';
import test from 'node:test';
import { createSessionId, sessionKey } from '../src/session.js';

test('session ids are random hex strings', () => {
  const first = createSessionId();
  const second = createSessionId();

  assert.match(first, /^[a-f0-9]{32}$/);
  assert.match(second, /^[a-f0-9]{32}$/);
  assert.notEqual(first, second);
});

test('session keys preserve the reader storage contract', () => {
  assert.equal(sessionKey('abc123'), 'speedReaderSession:abc123');
});
