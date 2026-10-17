// Live updates over Server-Sent Events. The server emits `change` whenever a
// project Markdown file changes; EventSource reconnects on its own.

import { byId } from './dom.js';

function setIndicator(mode, text) {
  byId('live').className = 'live ' + mode;
  byId('live-text').textContent = text;
}

export function connectLive(onChange) {
  const source = new EventSource('/api/events');
  source.addEventListener('ready', () => setIndicator('on', 'Live'));
  source.addEventListener('change', onChange);
  // Reload after reconnecting in case changes were missed while offline.
  source.onopen = onChange;
  source.onerror = () => setIndicator('off', 'Reconnecting…');
  return source;
}
