// Read-only client for the board server's JSON API.

import { state } from './state.js';

async function fetchJSON(url) {
  const response = await fetch(url, { cache: 'no-store' });
  if (!response.ok) throw new Error(response.status + ' ' + url);
  return response.json();
}

// Add a comment to a task. The server stores it outside the repository and
// agents read it with `board inbox`.
export async function postComment(projectId, taskNumber, body) {
  const url = '/api/projects/' + encodeURIComponent(projectId)
    + '/tasks/' + encodeURIComponent(taskNumber) + '/comments';
  const response = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
    body,
  });
  if (!response.ok) {
    let message = String(response.status);
    try { message = (await response.json()).error || message; } catch (_) {}
    throw new Error(message);
  }
  return response.json();
}

// Refresh the project list and the selected project's detail into `state`.
// Falls back to the newest project when the selection no longer exists.
export async function loadState() {
  const { repo, projects } = await fetchJSON('/api/projects');
  state.repo = repo;
  state.projects = projects;
  if (!projects.some((project) => project.id === state.projectId)) {
    state.projectId = projects.length ? projects[0].id : null;
  }
  state.project = state.projectId
    ? await fetchJSON('/api/projects/' + encodeURIComponent(state.projectId))
    : null;
}
