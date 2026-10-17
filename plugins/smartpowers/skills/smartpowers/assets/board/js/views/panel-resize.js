// Drag the task panel's left edge to resize it. The width is remembered per
// viewer; double-click the handle to reset. Arrow keys resize from the keyboard.
// On narrow screens the panel is full-width (see app.css) and this is inert.

import { byId } from '../dom.js';
import { readPref, writePref } from '../state.js';

const PREF = 'panelWidth';
const MIN_WIDTH = 320;
const MIN_MAIN_WIDTH = 360; // keep the board usable next to the panel
const KEY_STEP = 32;

const panel = () => byId('panel');

function maxWidth() {
  const sidebar = document.querySelector('.sidebar');
  const sidebarWidth = sidebar ? sidebar.getBoundingClientRect().width : 0;
  return Math.max(MIN_WIDTH, window.innerWidth - sidebarWidth - MIN_MAIN_WIDTH);
}

function clamp(width) {
  return Math.round(Math.min(Math.max(width, MIN_WIDTH), maxWidth()));
}

function currentWidth() {
  return panel().getBoundingClientRect().width;
}

function setWidth(width) {
  const clamped = clamp(width);
  panel().style.width = clamped + 'px';
  const handle = byId('panel-resizer');
  handle.setAttribute('aria-valuenow', String(clamped));
  handle.setAttribute('aria-valuemin', String(MIN_WIDTH));
  handle.setAttribute('aria-valuemax', String(Math.round(maxWidth())));
  return clamped;
}

function resetWidth() {
  panel().style.width = '';
  writePref(PREF, '');
}

function startDrag(event) {
  if (event.button !== 0) return;
  event.preventDefault();
  const handle = event.currentTarget;
  handle.setPointerCapture(event.pointerId);
  handle.classList.add('dragging');
  document.body.classList.add('resizing');

  // The panel is the rightmost column, so its width is the distance from
  // the pointer to the right edge of the window.
  const onMove = (move) => setWidth(window.innerWidth - move.clientX);
  const onEnd = () => {
    handle.removeEventListener('pointermove', onMove);
    handle.removeEventListener('pointerup', onEnd);
    handle.removeEventListener('pointercancel', onEnd);
    handle.classList.remove('dragging');
    document.body.classList.remove('resizing');
    writePref(PREF, String(Math.round(currentWidth())));
  };
  handle.addEventListener('pointermove', onMove);
  handle.addEventListener('pointerup', onEnd);
  handle.addEventListener('pointercancel', onEnd);
}

function onKeyDown(event) {
  const delta = { ArrowLeft: KEY_STEP, ArrowRight: -KEY_STEP }[event.key];
  if (delta === undefined) return;
  event.preventDefault();
  writePref(PREF, String(setWidth(currentWidth() + delta)));
}

export function initPanelResize() {
  const handle = byId('panel-resizer');
  handle.addEventListener('pointerdown', startDrag);
  handle.addEventListener('dblclick', resetWidth);
  handle.addEventListener('keydown', onKeyDown);

  const saved = Number(readPref(PREF));
  if (saved > 0) setWidth(saved);

  // Keep a remembered width within bounds when the window shrinks.
  // A hidden panel measures 0px, so only re-clamp while it is open.
  window.addEventListener('resize', () => {
    if (panel().style.width && !panel().hidden) setWidth(currentWidth());
  });
}
