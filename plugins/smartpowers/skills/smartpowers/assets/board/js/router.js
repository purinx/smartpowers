// Hash routes: #/<project-id>[/<task-number>][?view=board|list|flow].
// A `view` in the hash wins and becomes the stored preference; a hash
// without one keeps the stored preference, so older links still work.

import { isView, setView, state } from './state.js';

export function applyHash() {
  const [path, query = ''] = location.hash.replace(/^#\/?/, '').split('?');
  const parts = path.split('/').filter(Boolean);
  if (parts[0]) state.projectId = decodeURIComponent(parts[0]);
  state.taskNumber = parts[1] || null;
  const view = new URLSearchParams(query).get('view');
  if (isView(view)) setView(view);
}

function hashFor(projectId, taskNumber) {
  return '#/' + encodeURIComponent(projectId)
    + (taskNumber ? '/' + taskNumber : '')
    + '?view=' + state.view;
}

export function navigate(projectId, taskNumber) {
  const hash = hashFor(projectId, taskNumber);
  if (location.hash !== hash) location.hash = hash;
}

// Rewrite the hash after a view switch without adding a history entry, so a
// copied URL restores both the view and the open panel.
export function replaceViewInHash() {
  if (!state.projectId) return;
  const hash = hashFor(state.projectId, state.taskNumber);
  if (location.hash !== hash) history.replaceState(null, '', hash);
}

export function openTask(number) {
  navigate(state.projectId, number);
}

export function closeTask() {
  navigate(state.projectId);
}
