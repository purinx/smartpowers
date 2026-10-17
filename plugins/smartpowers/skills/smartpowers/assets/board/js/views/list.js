// List view: tasks grouped by status, one row each. Empty groups are hidden.

import { el, relativeTime } from '../dom.js';
import { openTask } from '../router.js';
import { columnsFor, statusIcon, tasksIn } from '../status.js';
import { remoteLink, taskChips, taskClasses } from './task-parts.js';

function row(task) {
  return el('div', { class: taskClasses('row', task), onclick: () => openTask(task.number) }, [
    statusIcon(task.statusKey),
    el('span', { class: 'mono muted' }, task.number),
    el('span', { class: 'row-title' }, task.title),
    el('span', { class: 'row-chips' }, [...taskChips(task, true), remoteLink(task)]),
    el('span', { class: 'row-date', title: task.updated }, relativeTime(task.updated)),
  ]);
}

function group(column, tasks) {
  const items = tasksIn(column, tasks);
  if (!items.length) return null;
  return el('div', { class: 'group' }, [
    el('div', { class: 'group-head' }, [
      statusIcon(column.key),
      el('span', null, column.label),
      el('span', { class: 'column-count' }, String(items.length)),
    ]),
    ...items.map(row),
  ]);
}

export function renderList(tasks) {
  return el('div', null, columnsFor(tasks).map((c) => group(c, tasks)));
}
