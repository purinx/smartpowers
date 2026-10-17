// Read-only client for the board server's JSON API.

import { state } from './state.js';

async function fetchJSON(url) {
  const response = await fetch(url, { cache: 'no-store' });
  if (!response.ok) throw new Error(response.status + ' ' + url);
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
