export const SESSION_KEY_PREFIX = 'speedReaderSession:';

export function createSessionId() {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('');
}

export function sessionKey(sessionId) {
  return `${SESSION_KEY_PREFIX}${sessionId}`;
}

export async function storeReaderSession(sessionId, session) {
  const key = sessionKey(sessionId);
  await storageSet(chrome.storage.local, { [key]: session });
  if (chrome.storage.session) {
    await storageSet(chrome.storage.session, { [key]: session }).catch(() => {});
  }
  return key;
}

function storageSet(storage, value) {
  try {
    const result = storage.set(value);
    if (result && typeof result.then === 'function') return result;
  } catch {
    // Fall through to callback-style APIs.
  }

  return new Promise((resolve, reject) => {
    storage.set(value, () => {
      const err = chrome.runtime?.lastError;
      if (err) reject(new Error(err.message));
      else resolve();
    });
  });
}
