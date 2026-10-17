// Project header: title, status, phase approvals, and overall progress.

import { byId, el } from '../dom.js';
import { state } from '../state.js';

function phaseChip(phase) {
  const status = phase.status || 'Missing';
  const tone = /^approved$/i.test(status) ? 'approved' : /^draft$/i.test(status) ? 'draft' : /^skipped$/i.test(status) ? 'skipped' : '';
  return el('span', { class: 'phase ' + tone, title: phase.file + ': ' + status }, [
    el('span', { class: 'phase-dot' }),
    phase.name,
  ]);
}

function renderProgress(project) {
  const total = project.taskCount;
  const done = project.counts.done || 0;
  const percent = total ? Math.round((done / total) * 100) : 0;
  byId('progress-fill').style.width = percent + '%';
  byId('progress-text').textContent = total ? `${done} / ${total} done · ${percent}%` : 'No tasks yet';
}

export function renderHeader() {
  const project = state.project;
  byId('project-header').hidden = !project;
  byId('crumb-title').textContent = project ? project.title : '—';
  document.title = project ? project.title + ' · Smartpowers' : 'Smartpowers Board';
  if (!project) return;

  byId('project-title').textContent = project.title;
  byId('project-status').textContent = project.status;
  byId('lead-agent').hidden = !project.agentListening;
  byId('project-id').textContent = project.id;
  byId('phases').replaceChildren(...project.phases.map(phaseChip));
  renderProgress(project);
}
