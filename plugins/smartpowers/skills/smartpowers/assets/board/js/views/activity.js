// Activity timeline for a task: the agent's progress log and comment
// threads, oldest first. User comments show whether the agent has them yet.

import { el, relativeTime } from '../dom.js';

const DELIVERY = {
  waiting: 'Waiting for agent',
  delivered: 'Sent to agent',
  addressed: 'Addressed',
};

function delivery(comment) {
  if (comment.status === 'addressed') return 'addressed';
  return comment.delivered ? 'delivered' : 'waiting';
}

function timestamp(iso) {
  return el('time', { class: 'timeline-time', datetime: iso, title: iso }, relativeTime(iso));
}

function avatar(author) {
  return el('span', { class: 'avatar ' + author, 'aria-hidden': 'true' }, author === 'agent' ? 'AI' : 'You');
}

function logItem(entry) {
  return el('li', { class: 'timeline-item log' }, [
    el('span', { class: 'timeline-dot', 'aria-hidden': 'true' }),
    el('div', { class: 'log-line' }, [el('span', { class: 'log-message' }, entry.message), timestamp(entry.time)]),
  ]);
}

function commentBlock(comment, extra) {
  const name = comment.author === 'agent' ? 'Agent' : 'You';
  return el('div', { class: 'comment ' + comment.author }, [
    avatar(comment.author),
    el('div', { class: 'comment-main' }, [
      el('div', { class: 'comment-head' }, [el('strong', null, name), timestamp(comment.created), extra]),
      // Plain text with preserved line breaks (white-space: pre-wrap).
      el('div', { class: 'comment-body' }, comment.body),
    ]),
  ]);
}

function threadItem(root, replies) {
  const state = delivery(root);
  const badge = root.author === 'user'
    ? el('span', { class: 'delivery ' + state }, DELIVERY[state])
    : null;
  return el('li', { class: 'timeline-item thread' }, [
    commentBlock(root, badge),
    ...replies.map((reply) => el('div', { class: 'reply' }, commentBlock(reply))),
  ]);
}

export function renderActivity(task) {
  const replies = new Map();
  const roots = [];
  for (const comment of task.comments) {
    if (comment.replyTo) {
      if (!replies.has(comment.replyTo)) replies.set(comment.replyTo, []);
      replies.get(comment.replyTo).push(comment);
    } else {
      roots.push(comment);
    }
  }
  const items = [
    ...task.log.map((entry) => ({ time: entry.time, node: () => logItem(entry) })),
    ...roots.map((root) => ({ time: root.created, node: () => threadItem(root, replies.get(root.id) || []) })),
  ];
  // Timestamps are RFC 3339 UTC, so they sort as text.
  items.sort((a, b) => (a.time < b.time ? -1 : a.time > b.time ? 1 : 0));
  if (!items.length) {
    return [el('li', { class: 'timeline-empty' }, 'No activity yet. Agent progress and your comments appear here.')];
  }
  return items.map((item) => item.node());
}

// User comments the agent hasn't addressed yet.
export function openComments(task) {
  return task.comments.filter((c) => c.author === 'user' && c.status === 'open');
}
