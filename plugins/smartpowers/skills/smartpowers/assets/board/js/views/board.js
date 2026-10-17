// Board view: one column per status, one card per task.

import { el } from '../dom.js';
import { openTask } from '../router.js';
import { columnsFor, statusIcon, tasksIn } from '../status.js';
import { remoteLink, taskChips, taskClasses } from './task-parts.js';

function card(task) {
  const node = el('div', {
    class: taskClasses('card', task),
    tabindex: '0',
    onclick: () => openTask(task.number),
    onkeydown: (event) => { if (event.key === 'Enter') openTask(task.number); },
  }, [
    el('div', { class: 'card-top' }, [el('span', { class: 'mono' }, task.number), remoteLink(task)]),
    el('div', { class: 'card-title' }, [statusIcon(task.statusKey), el('span', null, task.title)]),
  ]);
  const chips = taskChips(task, true);
  if (chips.length) node.appendChild(el('div', { class: 'card-meta' }, chips));
  return node;
}

function column(column, tasks) {
  const items = tasksIn(column, tasks);
  return el('div', { class: 'column' }, [
    el('div', { class: 'column-head' }, [
      statusIcon(column.key),
      el('span', null, column.label),
      el('span', { class: 'column-count' }, String(items.length)),
    ]),
    el('div', { class: 'cards' }, items.length ? items.map(card) : el('div', { class: 'column-empty' }, 'No tasks')),
  ]);
}

export function renderBoard(tasks) {
  return el('div', { class: 'board' }, columnsFor(tasks).map((c) => column(c, tasks)));
}
