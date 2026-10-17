// Main content area: empty states, or the board, list, or flow for the filtered tasks.

import { byId, el } from '../dom.js';
import { state } from '../state.js';
import { renderBoard } from './board.js';
import { renderFlow } from './flow.js';
import { renderList } from './list.js';

function emptyState(title, detail) {
  return el('div', { class: 'empty' }, [el('h3', null, title), el('div', null, detail)]);
}

// Case-insensitive match on number, title, status, remote key, and requirement IDs.
function filteredTasks() {
  const query = state.filter.trim().toLowerCase();
  const tasks = state.project.tasks;
  if (!query) return tasks;
  return tasks.filter((task) => [
    task.number, task.title, task.status, task.remote?.key ?? '', ...task.requirements,
  ].join(' ').toLowerCase().includes(query));
}

function contentNode() {
  if (!state.projects.length) {
    return emptyState('No Smartpowers projects', 'Projects appear here once docs/smartpowers/projects/<id>/ exists.');
  }
  if (!state.project) return null;
  if (!state.project.tasks.length) {
    return emptyState('No tasks yet', 'Tasks appear after the implementation plan creates tasks/NN_*.md files.');
  }
  const tasks = filteredTasks();
  if (state.view === 'list') return renderList(tasks);
  if (state.view === 'flow') return renderFlow(tasks, state.project.graphIssues || []);
  return renderBoard(tasks);
}

export function renderContent() {
  const node = contentNode();
  byId('content').replaceChildren(...(node ? [node] : []));
}
