import { writable, derived } from 'svelte/store';

// Auth state
export const user = writable(null);
export const isLoggedIn = derived(user, ($user) => $user !== null);

// Board data
export const boards = writable([]);
export const currentBoard = writable(null);
export const tasks = writable([]);
export const tags = writable([]);

// Shared board context
export const sharedContext = writable(null); // { permission, owner_username, token }

// UI state
export const flash = writable(null);
export const activeModal = writable(null); // 'task-create' | 'task-edit' | 'share' | 'tags' | null
export const editingTask = writable(null);

// Helper to show flash messages
let flashTimeout;
export function showFlash(message, kind = 'error') {
  flash.set({ message, kind });
  clearTimeout(flashTimeout);
  flashTimeout = setTimeout(() => flash.set(null), 4000);
}
