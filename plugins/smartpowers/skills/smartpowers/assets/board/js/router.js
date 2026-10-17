// Hash routes: #/<project-id> and #/<project-id>/<task-number>.

import { state } from './state.js';

export function applyHash() {
  const parts = location.hash.replace(/^#\/?/, '').split('/').filter(Boolean);
  if (parts[0]) state.projectId = decodeURIComponent(parts[0]);
  state.taskNumber = parts[1] || null;
}

export function navigate(projectId, taskNumber) {
  const hash = '#/' + encodeURIComponent(projectId) + (taskNumber ? '/' + taskNumber : '');
  if (location.hash !== hash) location.hash = hash;
}

export function openTask(number) {
  navigate(state.projectId, number);
}

export function closeTask() {
  navigate(state.projectId);
}
