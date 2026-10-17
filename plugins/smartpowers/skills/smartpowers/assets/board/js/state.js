// Shared UI state. main.js and the router change it; views only read it.

export const VIEWS = ['board', 'list', 'flow'];

export function isView(value) {
  return VIEWS.includes(value);
}

export const state = {
  repo: '',
  projects: [],
  project: null,
  projectId: null,
  taskNumber: null,
  view: isView(readPref('view')) ? readPref('view') : 'board',
  filter: '',
  // `${projectId}/${number}` -> `${statusKey}|${updated}` from the last render,
  // used to flash tasks that changed since then.
  previous: {},
};

// localStorage can be unavailable (private windows, blocked site data).
export function readPref(key) {
  try { return localStorage.getItem('smartpowers.' + key); } catch (_) { return null; }
}

export function writePref(key, value) {
  try { localStorage.setItem('smartpowers.' + key, value); } catch (_) {}
}

export function setView(view) {
  state.view = view;
  writePref('view', view);
}

export function currentTasks() {
  return state.project ? state.project.tasks : [];
}

export function taskByNumber(number) {
  return currentTasks().find((task) => task.number === number);
}

function snapshotKey(task) {
  return state.projectId + '/' + task.number;
}

export function changedSinceLastRender(task) {
  const previous = state.previous[snapshotKey(task)];
  return previous !== undefined && previous !== task.statusKey + '|' + task.updated;
}

export function rememberTaskSnapshots() {
  state.previous = {};
  for (const task of currentTasks()) {
    state.previous[snapshotKey(task)] = task.statusKey + '|' + task.updated;
  }
}
