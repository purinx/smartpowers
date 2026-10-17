// Sidebar: repository name and the project list with done/total counts.

import { byId, el } from '../dom.js';
import { navigate } from '../router.js';
import { state } from '../state.js';
import { projectStatusKey, statusIcon } from '../status.js';

function projectItem(project) {
  const done = project.counts.done || 0;
  return el('button', {
    class: 'nav-item' + (project.id === state.projectId ? ' active' : ''),
    title: project.id,
    onclick: () => navigate(project.id),
  }, [
    statusIcon(projectStatusKey(project.status)),
    el('span', { class: 'nav-title' }, project.title),
    el('span', { class: 'nav-count' }, project.taskCount ? done + '/' + project.taskCount : ''),
  ]);
}

export function renderSidebar() {
  byId('repo-name').textContent = state.repo || 'Smartpowers';
  const nav = byId('project-nav');
  nav.replaceChildren(...(state.projects.length
    ? state.projects.map(projectItem)
    : [el('div', { class: 'nav-empty' }, 'No projects yet')]));
}
