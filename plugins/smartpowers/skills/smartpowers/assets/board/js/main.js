// Entry point: wires routing, controls, keyboard shortcuts, and live reloads.
//
// Module map:
//   state.js       shared UI state and per-viewer preferences
//   api.js         fetches projects from the board server
//   router.js      #/<project>/<task> routes
//   live.js        Server-Sent Events subscription
//   status.js      status columns and icons
//   markdown.js    task body renderer
//   dom.js         element, escaping, and time helpers
//   views/*.js     one module per screen region

import { loadState } from './api.js';
import { byId } from './dom.js';
import { connectLive } from './live.js';
import { applyHash, closeTask } from './router.js';
import { rememberTaskSnapshots, setView, state } from './state.js';
import { renderContent } from './views/content.js';
import { renderHeader } from './views/header.js';
import { renderPanel } from './views/panel.js';
import { initPanelResize } from './views/panel-resize.js';
import { initComposer } from './views/composer.js';
import { renderSidebar } from './views/sidebar.js';

function render() {
  for (const button of document.querySelectorAll('.segmented button')) {
    button.classList.toggle('active', button.dataset.view === state.view);
  }
  renderSidebar();
  renderHeader();
  renderContent();
  renderPanel();
  rememberTaskSnapshots();
}

async function reload() {
  try {
    await loadState();
    render();
  } catch (error) {
    console.error(error);
  }
}

function switchView(view) {
  setView(view);
  render();
}

function onHashChange() {
  const before = state.projectId;
  applyHash();
  if (before !== state.projectId) {
    state.previous = {}; // don't flash every task of a newly opened project
    reload();
  } else {
    render();
  }
}

function onKeyDown(event) {
  const typing = /^(INPUT|TEXTAREA)$/.test(document.activeElement.tagName);
  if (event.key === 'Escape') {
    if (typing) document.activeElement.blur();
    else if (state.taskNumber) closeTask();
    return;
  }
  if (typing || event.metaKey || event.ctrlKey || event.altKey) return;
  if (event.key === '/') {
    event.preventDefault();
    byId('filter').focus();
  } else if (event.key === 'b') {
    switchView('board');
  } else if (event.key === 'l') {
    switchView('list');
  }
}

for (const button of document.querySelectorAll('.segmented button')) {
  button.addEventListener('click', () => switchView(button.dataset.view));
}
byId('filter').addEventListener('input', (event) => {
  state.filter = event.target.value;
  renderContent();
});
byId('panel-close').addEventListener('click', closeTask);
window.addEventListener('hashchange', onHashChange);
document.addEventListener('keydown', onKeyDown);
initPanelResize();
initComposer(reload); // show a new comment right away instead of waiting for the watcher

applyHash();
reload();
connectLive(reload);
