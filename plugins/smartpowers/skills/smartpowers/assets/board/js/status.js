// Task statuses: board columns and their Linear-style icons.
// Keys match `statusKey` from the server.

export const COLUMNS = [
  { key: 'planned', label: 'Planned' },
  { key: 'in_progress', label: 'In Progress' },
  { key: 'blocked', label: 'Blocked' },
  { key: 'done', label: 'Done' },
  { key: 'other', label: 'Other' },
];

const ICONS = {
  planned:
    '<circle cx="7" cy="7" r="5.5" fill="none" stroke="var(--planned)" stroke-width="1.5" stroke-dasharray="2.2 1.6"/>',
  in_progress:
    '<circle cx="7" cy="7" r="5.5" fill="none" stroke="var(--progress)" stroke-width="1.5"/>' +
    '<path d="M7 3.5a3.5 3.5 0 0 1 0 7z" fill="var(--progress)"/>',
  blocked:
    '<circle cx="7" cy="7" r="6" fill="var(--blocked)"/>' +
    '<path d="M4.5 7h5" stroke="#fff" stroke-width="1.6" stroke-linecap="round"/>',
  done:
    '<circle cx="7" cy="7" r="6" fill="var(--done)"/>' +
    '<path d="M4.4 7.2l1.8 1.8 3.4-3.6" stroke="var(--on-accent)" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>',
  other: '<circle cx="7" cy="7" r="5.5" fill="none" stroke="var(--other)" stroke-width="1.5"/>',
};

export function statusIcon(key) {
  const span = document.createElement('span');
  span.className = 'status-icon';
  span.setAttribute('aria-hidden', 'true');
  span.innerHTML = '<svg viewBox="0 0 14 14" width="14" height="14">' + (ICONS[key] || '') + '</svg>';
  return span;
}

// The "Other" column only appears when some task has an unrecognized status.
export function columnsFor(tasks) {
  return COLUMNS.filter((column) =>
    column.key !== 'other' || tasks.some((task) => task.statusKey === 'other'));
}

export function tasksIn(column, tasks) {
  return tasks.filter((task) => task.statusKey === column.key);
}

// Map PROJECT.md `Status:` onto a task status for the sidebar icon.
export function projectStatusKey(status) {
  switch (String(status || '').toLowerCase()) {
    case 'completed':
    case 'done':
      return 'done';
    case 'blocked':
      return 'blocked';
    case 'active':
    case 'in progress':
      return 'in_progress';
    default:
      return 'planned';
  }
}
