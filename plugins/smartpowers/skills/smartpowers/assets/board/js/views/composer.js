// Comment composer in the task panel. Its markup is static (index.html) so
// live re-renders never wipe what the user is typing. Unsent drafts are kept
// per task while switching between tasks.

import { postComment } from '../api.js';
import { byId } from '../dom.js';
import { state } from '../state.js';

const drafts = new Map();
let currentKey = null;
let afterPost = () => {};

function setStatus(text, isError = false) {
  const status = byId('comment-status');
  status.textContent = text;
  status.classList.toggle('error', isError);
}

async function submit() {
  const input = byId('comment-input');
  const body = input.value.trim();
  if (!body || !currentKey) return;
  // Capture the target now; the user may switch tasks while this is in flight.
  const key = currentKey;
  const [projectId, taskNumber] = [state.projectId, state.taskNumber];
  const button = byId('comment-form').querySelector('button[type="submit"]');
  button.disabled = true;
  setStatus('Sending…');
  try {
    await postComment(projectId, taskNumber, body);
    drafts.delete(key);
    if (key === currentKey) {
      input.value = '';
      setStatus('Sent. The agent picks it up while it works.');
    }
    afterPost();
  } catch (error) {
    if (key === currentKey) setStatus('Could not send: ' + error.message, true);
  } finally {
    button.disabled = false;
  }
}

export function initComposer(onPosted) {
  afterPost = onPosted;
  byId('comment-form').addEventListener('submit', (event) => {
    event.preventDefault();
    submit();
  });
  byId('comment-input').addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      submit();
    }
  });
}

// Call on every panel render: swaps drafts when the selected task changes.
export function syncComposer() {
  const key = state.taskNumber ? state.projectId + '/' + state.taskNumber : null;
  if (key === currentKey) return;
  const input = byId('comment-input');
  if (currentKey) drafts.set(currentKey, input.value);
  currentKey = key;
  input.value = (key && drafts.get(key)) || '';
  setStatus('');
}
