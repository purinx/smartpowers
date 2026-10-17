// Task detail panel: properties plus the rendered task file body.

import { byId, el, relativeTime } from '../dom.js';
import { renderMarkdown, taskBody } from '../markdown.js';
import { openTask } from '../router.js';
import { state, taskByNumber } from '../state.js';
import { statusIcon } from '../status.js';
import { renderActivity } from './activity.js';
import { syncComposer } from './composer.js';

function dependencyChips(task) {
  if (!task.dependsOn.length) return 'None';
  return task.dependsOn.map((number) => {
    const dependency = taskByNumber(number);
    return el('span', { class: 'chip', style: 'cursor:pointer', onclick: () => openTask(number) }, [
      dependency ? statusIcon(dependency.statusKey) : null,
      number + (dependency ? ' ' + dependency.title : ''),
    ]);
  });
}

function remoteValue(task) {
  if (!task.remote) return el('span', { class: 'muted' }, 'Not linked');
  if (!task.remote.url) return task.remote.raw;
  return el('a', { href: task.remote.url, target: '_blank', rel: 'noopener' }, task.remote.key || task.remote.url);
}

function properties(task) {
  const requirements = task.requirements.length
    ? task.requirements.map((id) => el('span', { class: 'chip' }, id))
    : '—';
  return [
    ['Status', [statusIcon(task.statusKey), task.status]],
    ['Requirements', requirements],
    ['Depends on', dependencyChips(task)],
    ['Parallel group', task.parallelGroup || '—'],
    ['Remote task', remoteValue(task)],
    ['File', el('code', { class: 'mono' }, state.project.path + '/tasks/' + task.file)],
    ['Updated', relativeTime(task.updated)],
  ].flatMap(([label, value]) => [el('dt', null, label), el('dd', null, value)]);
}

export function renderPanel() {
  const task = state.taskNumber && taskByNumber(state.taskNumber);
  byId('panel').hidden = !task;
  syncComposer();
  if (!task) return;
  byId('panel-id').textContent = state.project.id + ' / ' + task.number;
  byId('panel-title').textContent = task.title;
  byId('panel-props').replaceChildren(...properties(task));
  // renderMarkdown escapes its input before adding tags.
  byId('panel-body').innerHTML = renderMarkdown(taskBody(task.body));
  byId('panel-activity').replaceChildren(...renderActivity(task));
}
