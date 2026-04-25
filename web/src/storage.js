export const LS_KEY = {
  font: 'sr.font',
  wpm: 'sr.wpm',
  theme: 'sr.themeIdx',
  weather: 'sr.weatherIdx',
  preset: 'sr.preset',
  sessions: 'sr.sessions',
  bookmarks: 'sr.bookmarks',
};

const MAX_SESSION_TEXT_CHARS = 100_000;

export function lsGet(key, fallback) {
  try {
    const value = localStorage.getItem(key);
    return value === null ? fallback : value;
  } catch {
    return fallback;
  }
}

export function lsSet(key, value) {
  try {
    localStorage.setItem(key, String(value));
  } catch {}
}

export async function getFileHash(data) {
  if (!crypto.subtle) {
    let hash = 0;
    const bytes = new Uint8Array(data);
    for (let i = 0; i < bytes.length; i++) {
      hash = ((hash << 5) - hash) + bytes[i];
      hash |= 0;
    }
    return 'fb-' + Math.abs(hash).toString(16);
  }
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
}

export function getSavedLocation(hash) {
  return parseInt(lsGet(`sr.loc.${hash}`, '0'), 10);
}

export function saveLocation(hash, index) {
  if (!hash) return;
  lsSet(`sr.loc.${hash}`, index);
}

export function listSessions(limit = 6) {
  return readJson(LS_KEY.sessions, [])
    .filter(session => session && session.hash && session.title)
    .sort((a, b) => Number(b.updatedAt || 0) - Number(a.updatedAt || 0))
    .slice(0, limit);
}

export function upsertSession(session) {
  if (!session?.hash) return;
  const sessions = readJson(LS_KEY.sessions, []).filter(item => item.hash !== session.hash);
  const text = typeof session.text === 'string' && session.text.length <= MAX_SESSION_TEXT_CHARS
    ? session.text
    : undefined;
  sessions.unshift({
    ...session,
    text,
    updatedAt: Date.now(),
  });
  writeJson(LS_KEY.sessions, sessions.slice(0, 12));
}

export function removeSession(hash) {
  writeJson(LS_KEY.sessions, readJson(LS_KEY.sessions, []).filter(session => session.hash !== hash));
}

export function listBookmarks(hash) {
  if (!hash) return [];
  const all = readJson(LS_KEY.bookmarks, {});
  return Array.isArray(all[hash])
    ? all[hash].slice().sort((a, b) => Number(a.index || 0) - Number(b.index || 0))
    : [];
}

export function isBookmarked(hash, index) {
  return listBookmarks(hash).some(bookmark => bookmark.index === index);
}

export function toggleBookmark(hash, bookmark) {
  if (!hash || !Number.isInteger(bookmark?.index)) return [];
  const all = readJson(LS_KEY.bookmarks, {});
  const current = Array.isArray(all[hash]) ? all[hash] : [];
  const existing = current.findIndex(item => item.index === bookmark.index);
  all[hash] = existing >= 0
    ? current.filter((_, index) => index !== existing)
    : [...current, { ...bookmark, createdAt: Date.now() }];
  writeJson(LS_KEY.bookmarks, all);
  return listBookmarks(hash);
}

export function removeBookmark(hash, index) {
  if (!hash) return [];
  const all = readJson(LS_KEY.bookmarks, {});
  all[hash] = (Array.isArray(all[hash]) ? all[hash] : []).filter(bookmark => bookmark.index !== index);
  writeJson(LS_KEY.bookmarks, all);
  return listBookmarks(hash);
}

function readJson(key, fallback) {
  try {
    const raw = localStorage.getItem(key);
    return raw ? JSON.parse(raw) : fallback;
  } catch {
    return fallback;
  }
}

function writeJson(key, value) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {}
}
