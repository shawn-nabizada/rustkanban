import { api, ApiError } from './api.js';
import { user, boards, tasks, tags, currentBoard } from './stores.js';

export async function checkAuth() {
  try {
    return await api.getMe();
  } catch (e) {
    if (e instanceof ApiError && e.status === 401) return null;
    throw e;
  }
}

export function redirectToLogin() {
  window.location.href = `/login?redirect_url=${encodeURIComponent('/')}`;
}

export async function logout() {
  try { await api.logout(); } catch (_) { /* ignore */ }
  user.set(null);
  boards.set([]);
  tasks.set([]);
  tags.set([]);
  currentBoard.set(null);
}
