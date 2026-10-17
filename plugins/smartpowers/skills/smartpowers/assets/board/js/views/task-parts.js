// Pieces shared by the board, list, and detail panel.

import { el } from '../dom.js';
import { state, taskByNumber, changedSinceLastRender } from '../state.js';
import { openComments } from './activity.js';

// Dependencies that are not Done yet.
export function waitingOn(task) {
  if (task.statusKey === 'done') return [];
  return task.dependsOn.filter((number) => {
    const dependency = taskByNumber(number);
    return dependency && dependency.statusKey !== 'done';
  });
}

// Waiting dependencies, parallel group, and requirement IDs as chips.
// `compact` shows at most two requirement IDs plus a "+N" overflow chip.
export function taskChips(task, compact) {
  const chips = [];
  if (task.listening) {
    chips.push(el('span', { class: 'chip agent-live', title: 'An agent is working on this task and receives comments instantly' }, [
      el('span', { class: 'live-pulse', 'aria-hidden': 'true' }), 'Agent',
    ]));
  }
  const open = openComments(task).length;
  if (open) {
    chips.push(el('span', { class: 'chip comments', title: 'Open comments for the agent' }, '💬 ' + open));
  }
  const waiting = waitingOn(task);
  if (waiting.length) {
    chips.push(el('span', { class: 'chip warn', title: 'Waiting on unfinished dependencies' }, '⧗ ' + waiting.join(', ')));
  }
  if (task.parallelGroup) {
    chips.push(el('span', { class: 'chip', title: 'Parallel group' }, '◇ ' + task.parallelGroup));
  }
  const shown = compact ? task.requirements.slice(0, 2) : task.requirements;
  for (const id of shown) chips.push(el('span', { class: 'chip' }, id));
  const hidden = task.requirements.length - shown.length;
  if (hidden > 0) chips.push(el('span', { class: 'chip' }, '+' + hidden));
  return chips;
}

// The remote issue key (e.g. ENG-42), linked when the task file has a URL.
export function remoteLink(task) {
  if (!task.remote) return null;
  const label = task.remote.key || 'Remote';
  if (!task.remote.url) return el('span', { class: 'remote-key' }, label);
  return el('a', {
    class: 'remote-key',
    href: task.remote.url,
    target: '_blank',
    rel: 'noopener',
    onclick: (event) => event.stopPropagation(), // don't also open the panel
  }, label);
}

// The agent's latest progress entry, shown on cards of tasks in progress.
export function latestLog(task) {
  if (task.statusKey !== 'in_progress' || !task.log.length) return null;
  const entry = task.log[task.log.length - 1];
  return el('div', { class: 'card-log', title: entry.message }, entry.message);
}

// CSS classes for a clickable task row or card.
export function taskClasses(base, task) {
  return base
    + (task.number === state.taskNumber ? ' selected' : '')
    + (changedSinceLastRender(task) ? ' flash' : '');
}
