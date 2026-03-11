const BASE = '/api/v1';

class ApiError extends Error {
  constructor(status, data) {
    super(data?.message || data?.error || 'Request failed');
    this.status = status;
    this.data = data;
  }
}

async function request(method, path, body = null) {
  const opts = {
    method,
    headers: { 'Content-Type': 'application/json' },
    credentials: 'same-origin',
  };
  if (body) opts.body = JSON.stringify(body);
  const resp = await fetch(`${BASE}${path}`, opts);
  if (resp.status === 204) return null;
  const data = await resp.json().catch(() => null);
  if (!resp.ok) throw new ApiError(resp.status, data);
  return data;
}

export const api = {
  getMe: () => request('GET', '/me'),
  listBoards: () => request('GET', '/boards'),
  getBoard: (uuid) => request('GET', `/boards/${uuid}`),

  createTask: (body) => request('POST', '/tasks', body),
  updateTask: (uuid, body) => request('PATCH', `/tasks/${uuid}`, body),
  deleteTask: (uuid) => request('DELETE', `/tasks/${uuid}`),

  createTag: (body) => request('POST', '/tags', body),
  updateTag: (uuid, body) => request('PATCH', `/tags/${uuid}`, body),
  deleteTag: (uuid) => request('DELETE', `/tags/${uuid}`),

  createShare: (boardUuid, body) => request('POST', `/boards/${boardUuid}/shares`, body),
  listShares: (boardUuid) => request('GET', `/boards/${boardUuid}/shares`),
  deleteShare: (id) => request('DELETE', `/shares/${id}`),

  getSharedBoard: (token) => request('GET', `/shared/${token}`),
  createSharedTask: (token, body) => request('POST', `/shared/${token}/tasks`, body),
  updateSharedTask: (token, uuid, body) => request('PATCH', `/shared/${token}/tasks/${uuid}`, body),
  deleteSharedTask: (token, uuid) => request('DELETE', `/shared/${token}/tasks/${uuid}`),

  // Account management
  listDevices: () => request('GET', '/account/devices'),
  renameDevice: (id, body) => request('PATCH', `/account/devices/${id}`, body),
  revokeDevice: (id) => request('DELETE', `/account/devices/${id}`),
  listTokens: () => request('GET', '/account/tokens'),
  createToken: (body) => request('POST', '/account/tokens', body),
  revokeToken: (id) => request('DELETE', `/account/tokens/${id}`),
  deleteAccount: () => request('DELETE', '/account'),
  logout: () => request('POST', '/auth/logout'),
};

export { ApiError };
