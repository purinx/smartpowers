// Flow view: the task dependency graph, flowing left to right by depth.
//
// The server computes `depth`, `ready`, and `graphIssues`; this module only
// lays tasks out and draws them. It never decides readiness itself.
//
// Layout: one column per depth. The order inside each column comes from two
// barycenter sweeps, and the vertical position from a fixed number of
// relaxation rounds that pull each task toward the middle of the tasks it
// connects to, so arrows run nearly straight. Nothing is random, so the same
// tasks always give the same picture. Nodes are fixed-size HTML cards; one
// inline <svg> beneath them draws the edges. The whole drawing sits in a
// viewport that fits it to the available space, pans with the wheel or by
// dragging the background, and zooms with Ctrl/Cmd + wheel or a pinch.

import { el } from '../dom.js';
import { openTask } from '../router.js';
import { state } from '../state.js';
import { statusIcon } from '../status.js';
import { taskClasses } from './task-parts.js';

const NODE_WIDTH = 232;
const NODE_HEIGHT = 76;
const COLUMN_GAP = 128;
const NODE_GAP = 44;
const PADDING = 24;
const RELAX_ROUNDS = 12;
const LANE_MARGIN = 8;
const LANE_SPREAD = 6;
const FIT_MARGIN = 32;
const FIT_SCALE = [0.4, 1.5];
const ZOOM_SCALE = [0.25, 2.5];
const GRID = 20;
const SVG_NS = 'http://www.w3.org/2000/svg';

// Pan and zoom per project, kept while live updates redraw the same graph.
// Opening another project starts it from the fitted view again.
const views = new Map();
let shownProject = null;

const mean = (values) => values.reduce((sum, v) => sum + v, 0) / values.length;
const clamp = (value, [low, high]) => Math.min(high, Math.max(low, value));

// --- Layout ---------------------------------------------------------------

function buildGraph(tasks) {
  const byNumber = new Map(tasks.map((task) => [task.number, task]));
  const preds = new Map(tasks.map((task) => [
    task.number, [...new Set(task.dependsOn)].filter((n) => byNumber.has(n) && n !== task.number),
  ]));
  const succs = new Map(tasks.map((task) => [task.number, []]));
  for (const [number, list] of preds) for (const p of list) succs.get(p).push(number);

  const byDepth = new Map();
  for (const task of tasks) {
    const depth = task.depth ?? 0;
    if (!byDepth.has(depth)) byDepth.set(depth, []);
    byDepth.get(depth).push(task.number); // API order is plan order
  }
  const columns = [...byDepth.keys()].sort((a, b) => a - b).map((depth) => byDepth.get(depth));
  const column = new Map();
  columns.forEach((numbers, c) => numbers.forEach((n) => column.set(n, c)));
  return { byNumber, preds, succs, columns, column };
}

// Two barycenter sweeps (left to right, then right to left) order each
// column by the centered rank of its neighbors. Ties keep the previous order.
function orderColumns({ columns, preds, succs }) {
  const rank = new Map();
  const remember = (numbers) => numbers.forEach((n, i) => rank.set(n, i - (numbers.length - 1) / 2));
  columns.forEach(remember);
  const sweep = (indexes, neighbors) => {
    for (const c of indexes) {
      const keyed = columns[c].map((n, order) => {
        const linked = neighbors.get(n).filter((m) => rank.has(m)).map((m) => rank.get(m));
        return { n, order, key: linked.length ? mean(linked) : rank.get(n) };
      });
      keyed.sort((a, b) => a.key - b.key || a.order - b.order);
      columns[c] = keyed.map((entry) => entry.n);
      remember(columns[c]);
    }
  };
  const forward = columns.map((_, c) => c).slice(1);
  const backward = columns.map((_, c) => c).reverse().slice(1);
  for (let round = 0; round < 2; round += 1) {
    sweep(forward, preds);
    sweep(backward, succs);
  }
}

// Place a column's nodes as close to their targets as their order allows:
// the mean of the lowest and highest positions that keep the gaps. A node
// without a target (no links) packs against its neighbors.
function placeColumn(targets, current) {
  if (targets.every((t) => t === null)) return current;
  const step = NODE_HEIGHT + NODE_GAP;
  const down = [];
  targets.forEach((t, i) => {
    const after = i && down[i - 1] !== null ? down[i - 1] + step : null;
    down[i] = t === null ? after : Math.max(t, after ?? -Infinity);
  });
  const up = [];
  for (let i = targets.length - 1; i >= 0; i -= 1) {
    const before = i < targets.length - 1 && up[i + 1] !== null ? up[i + 1] - step : null;
    up[i] = targets[i] === null ? before : Math.min(targets[i], before ?? Infinity);
  }
  const placed = targets.map((t, i) => (t === null || down[i] === null || up[i] === null
    ? null : (down[i] + up[i]) / 2));
  // Pack unlinked nodes against the final position of the node above (or,
  // at the top, below), then restore the gaps in order.
  for (let i = 0; i < placed.length; i += 1) {
    if (placed[i] === null && i && placed[i - 1] !== null) placed[i] = placed[i - 1] + step;
  }
  for (let i = placed.length - 1; i >= 0; i -= 1) {
    if (placed[i] === null) placed[i] = placed[i + 1] - step;
  }
  for (let i = 1; i < placed.length; i += 1) placed[i] = Math.max(placed[i], placed[i - 1] + step);
  return placed;
}

// Vertical centers: start with each column centered, then relax each node
// toward the mean center of the tasks it connects to.
function relax({ columns, preds, succs }) {
  const center = new Map();
  for (const numbers of columns) {
    numbers.forEach((n, i) => center.set(n, (i - (numbers.length - 1) / 2) * (NODE_HEIGHT + NODE_GAP)));
  }
  for (let round = 0; round < RELAX_ROUNDS; round += 1) {
    const indexes = columns.map((_, c) => c);
    if (round % 2) indexes.reverse();
    for (const c of indexes) {
      const targets = columns[c].map((n) => {
        const linked = [...preds.get(n), ...succs.get(n)];
        return linked.length ? mean(linked.map((m) => center.get(m))) : null;
      });
      placeColumn(targets, columns[c].map((n) => center.get(n))).forEach((y, i) => center.set(columns[c][i], y));
    }
  }
  return center;
}

// --- Edges ----------------------------------------------------------------

// Free vertical spans of a column, between and around its nodes.
function freeSlots(numbers, top) {
  const slots = [];
  let previous = -Infinity;
  for (const n of numbers) {
    const y = top.get(n);
    slots.push([previous, y - LANE_MARGIN]);
    previous = y + NODE_HEIGHT + LANE_MARGIN;
  }
  slots.push([previous, Infinity]);
  return slots.filter(([low, high]) => high >= low);
}

// The y an edge takes through a skipped column: where the straight line
// would cross it if that is free, or else the nearest free point.
function laneThrough(numbers, top, desired, used) {
  const slots = freeSlots(numbers, top);
  let best = null;
  for (const [low, high] of slots) {
    const y = Math.min(high, Math.max(low, desired));
    if (!best || Math.abs(y - desired) < Math.abs(best.y - desired)) best = { y, low, high };
  }
  // Spread edges that share a slot so they don't merge into one line.
  const key = Math.round(best.low) + ':' + Math.round(best.high);
  const count = used.get(key) || 0;
  used.set(key, count + 1);
  const offset = (count % 2 ? -1 : 1) * Math.ceil(count / 2) * LANE_SPREAD;
  return Math.min(best.high, Math.max(best.low, best.y + offset));
}

// A smooth path through points, leaving and entering each point horizontally.
function smoothPath(points) {
  let d = `M${points[0][0]} ${points[0][1]}`;
  for (let i = 1; i < points.length; i += 1) {
    const [x0, y0] = points[i - 1];
    const [x1, y1] = points[i];
    const bend = Math.max(24, (x1 - x0) / 2);
    d += y0 === y1 ? `L${x1} ${y1}` : `C${x0 + bend} ${y0} ${x1 - bend} ${y1} ${x1} ${y1}`;
  }
  return d;
}

// Edge geometry as point lists, from the right middle of each dependency to
// the left middle of its dependent. Missing dependencies get no edge.
function routeEdges(graph, x, top) {
  const edges = [];
  const usedSlots = new Map();
  for (const [to, list] of graph.preds) {
    for (const from of list) {
      const fromColumn = graph.column.get(from);
      const toColumn = graph.column.get(to);
      const start = [x.get(from) + NODE_WIDTH, top.get(from) + NODE_HEIGHT / 2];
      const end = [x.get(to) - 1, top.get(to) + NODE_HEIGHT / 2];
      if (toColumn > fromColumn) {
        const points = [start];
        for (let c = fromColumn + 1; c < toColumn; c += 1) {
          const left = c * (NODE_WIDTH + COLUMN_GAP);
          const share = (left + NODE_WIDTH / 2 - start[0]) / (end[0] - start[0]);
          const lane = laneThrough(graph.columns[c], top, start[1] + share * (end[1] - start[1]), usedSlots);
          points.push([left - 1, lane], [left + NODE_WIDTH + 1, lane]);
        }
        points.push(end);
        edges.push({ from, to, points });
      } else {
        // Only a cycle puts a dependency in the same or a later column: loop
        // out to the right and come back into the dependent's right side.
        const back = [x.get(to) + NODE_WIDTH + 1, end[1]];
        const reach = Math.max(start[0], back[0]) + COLUMN_GAP / 2;
        edges.push({ from, to, loop: true, points: [start, [reach, (start[1] + back[1]) / 2], back] });
      }
    }
  }
  return edges;
}

function layout(tasks) {
  const graph = buildGraph(tasks);
  orderColumns(graph);
  const center = relax(graph);
  const x = new Map();
  const top = new Map();
  graph.columns.forEach((numbers, c) => numbers.forEach((n) => {
    x.set(n, c * (NODE_WIDTH + COLUMN_GAP));
    top.set(n, center.get(n) - NODE_HEIGHT / 2);
  }));
  const edges = routeEdges(graph, x, top);

  // Shift everything so the drawing starts at the padding.
  const ys = [...top.values()].flatMap((y) => [y, y + NODE_HEIGHT]);
  const xs = [...x.values()].map((v) => v + NODE_WIDTH);
  for (const edge of edges) for (const [px, py] of edge.points) { xs.push(px); ys.push(py); }
  const dy = PADDING - Math.min(...ys);
  for (const n of top.keys()) { top.set(n, top.get(n) + dy); x.set(n, x.get(n) + PADDING); }
  for (const edge of edges) edge.points = edge.points.map(([px, py]) => [px + PADDING, py + dy]);
  return {
    graph, x, top, edges,
    width: Math.max(...xs) + PADDING * 2,
    height: Math.max(...ys) - Math.min(...ys) + PADDING * 2,
  };
}

// --- Drawing --------------------------------------------------------------

function svgEl(tag, attrs, children) {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [name, value] of Object.entries(attrs || {})) node.setAttribute(name, value);
  for (const child of children || []) node.appendChild(child);
  return node;
}

function arrowMarker(id, className) {
  return svgEl('marker', {
    id, class: className, viewBox: '0 0 10 10', refX: '9', refY: '5',
    markerWidth: '7', markerHeight: '7', orient: 'auto-start-reverse',
  }, [svgEl('path', { d: 'M0 0L10 5L0 10z' })]);
}

function edgeLayer({ edges, graph, width, height }) {
  const paths = edges.map(({ from, to, points, loop }) => {
    const done = graph.byNumber.get(from).statusKey === 'done';
    const d = loop
      ? `M${points[0][0]} ${points[0][1]}Q${points[1][0]} ${points[0][1]} ${points[1][0]} ${points[1][1]}`
        + `Q${points[1][0]} ${points[2][1]} ${points[2][0]} ${points[2][1]}`
      : smoothPath(points);
    return svgEl('path', {
      class: 'flow-edge ' + (done ? 'done' : 'pending'),
      d,
      'data-from': from,
      'data-to': to,
      'marker-end': done ? 'url(#flow-arrow-done)' : 'url(#flow-arrow-pending)',
    });
  });
  return svgEl('svg', {
    class: 'flow-edges', width: String(width), height: String(height), 'aria-hidden': 'true',
  }, [
    svgEl('defs', null, [
      arrowMarker('flow-arrow-done', 'flow-arrow done'),
      arrowMarker('flow-arrow-pending', 'flow-arrow pending'),
    ]),
    ...paths,
  ]);
}

// Emphasize the edges touching a task and fade the rest.
function highlight(viewport, number) {
  viewport.classList.toggle('tracing', number !== null);
  for (const path of viewport.querySelectorAll('.flow-edge')) {
    path.classList.toggle('highlight', number !== null
      && (path.dataset.from === number || path.dataset.to === number));
  }
}

function node(task, x, y, viewport) {
  const chips = [];
  if (task.ready) {
    chips.push(el('span', { class: 'chip ready', title: 'All dependencies are Done; this task can start' }, [
      el('span', { 'aria-hidden': 'true' }, '▶'), 'Ready',
    ]));
  }
  if (task.listening) {
    chips.push(el('span', { class: 'chip agent-live', title: 'An agent is working on this task and receives comments instantly' }, [
      el('span', { class: 'live-pulse', 'aria-hidden': 'true' }), 'Agent',
    ]));
  }
  const trace = () => highlight(viewport(), task.number);
  const untrace = () => highlight(viewport(), null);
  return el('div', {
    class: taskClasses('flow-node status-' + task.statusKey + (task.ready ? ' ready' : ''), task),
    style: `left:${x}px;top:${y}px`,
    tabindex: '0',
    role: 'button',
    title: task.number + ' ' + task.title,
    onclick: () => openTask(task.number),
    onkeydown: (event) => { if (event.key === 'Enter') openTask(task.number); },
    onmouseenter: trace,
    onmouseleave: untrace,
    onfocus: trace,
    onblur: untrace,
  }, [
    el('div', { class: 'flow-node-top' }, [
      el('span', { class: 'mono flow-node-number' }, task.number),
      el('span', { class: 'flow-node-status' }, [statusIcon(task.statusKey), el('span', null, task.status)]),
      ...chips,
    ]),
    el('div', { class: 'flow-node-title' }, task.title),
  ]);
}

function issueText(issue) {
  if (issue.kind === 'missingDependency') {
    return `Task ${issue.task} depends on missing task ${issue.dependency}`;
  }
  if (issue.kind === 'cycle') return 'Dependency cycle among tasks: ' + issue.tasks.join(', ');
  return JSON.stringify(issue);
}

function issueBanner(issues) {
  if (!issues.length) return null;
  return el('div', { class: 'flow-issues', role: 'status' }, [
    el('div', { class: 'flow-issues-title' }, [
      el('span', { 'aria-hidden': 'true' }, '⚠'),
      'The dependency graph has problems; columns may be misleading',
    ]),
    el('ul', null, issues.map((issue) => el('li', null, issueText(issue)))),
  ]);
}

// --- Pan and zoom ---------------------------------------------------------

function applyView(viewport, canvas, view) {
  canvas.style.transform = `translate(${view.x}px, ${view.y}px) scale(${view.scale})`;
  viewport.style.backgroundSize = `${GRID * view.scale}px ${GRID * view.scale}px`;
  viewport.style.backgroundPosition = `${view.x}px ${view.y}px`;
}

// The largest scale that shows the whole drawing, centered.
function fitView(viewport, width, height) {
  const room = [viewport.clientWidth - FIT_MARGIN * 2, viewport.clientHeight - FIT_MARGIN * 2];
  const scale = clamp(Math.min(room[0] / width, room[1] / height), FIT_SCALE);
  const offset = (space, size) => (space - size * scale > 0 ? (space - size * scale) / 2 : FIT_MARGIN);
  return { scale, x: offset(viewport.clientWidth, width), y: offset(viewport.clientHeight, height) };
}

function enablePanZoom(viewport, canvas, key, width, height, signature) {
  const remembered = views.get(key);
  let view = remembered && remembered.signature === signature ? remembered : null;
  const store = (next) => {
    view = { ...next, signature };
    views.set(key, view);
    applyView(viewport, canvas, view);
  };
  const fit = () => store(fitView(viewport, width, height));
  if (view) applyView(viewport, canvas, view);
  else requestAnimationFrame(() => { if (viewport.isConnected) fit(); });

  // Scrolling pans; Ctrl/Cmd + wheel (and trackpad pinch, which browsers
  // report as a Ctrl + wheel) zooms around the cursor.
  viewport.addEventListener('wheel', (event) => {
    if (!view) return;
    event.preventDefault();
    if (!event.ctrlKey && !event.metaKey) {
      store({ scale: view.scale, x: view.x - event.deltaX, y: view.y - event.deltaY });
      return;
    }
    const bounds = viewport.getBoundingClientRect();
    const [mx, my] = [event.clientX - bounds.left, event.clientY - bounds.top];
    const scale = clamp(view.scale * Math.exp(-event.deltaY * 0.0015), ZOOM_SCALE);
    const ratio = scale / view.scale;
    store({ scale, x: mx - (mx - view.x) * ratio, y: my - (my - view.y) * ratio });
  }, { passive: false });

  viewport.addEventListener('pointerdown', (event) => {
    if (!view || event.button !== 0 || event.target.closest('.flow-node, .flow-fit')) return;
    const start = { px: event.clientX, py: event.clientY, x: view.x, y: view.y };
    viewport.setPointerCapture(event.pointerId);
    viewport.classList.add('panning');
    const move = (e) => store({ scale: view.scale, x: start.x + e.clientX - start.px, y: start.y + e.clientY - start.py });
    const stop = () => {
      viewport.classList.remove('panning');
      viewport.removeEventListener('pointermove', move);
      viewport.removeEventListener('pointerup', stop);
      viewport.removeEventListener('pointercancel', stop);
    };
    viewport.addEventListener('pointermove', move);
    viewport.addEventListener('pointerup', stop);
    viewport.addEventListener('pointercancel', stop);
  });
  return fit;
}

export function renderFlow(tasks, issues) {
  if (!tasks.length) {
    return el('div', { class: 'flow' }, [issueBanner(issues), el('div', { class: 'column-empty' }, 'No matching tasks')]);
  }
  const drawing = layout(tasks);
  let viewport = null;
  const nodes = drawing.graph.columns.flat() // DOM order = column order, so Tab follows columns
    .map((n) => node(drawing.graph.byNumber.get(n), drawing.x.get(n), drawing.top.get(n), () => viewport));
  const canvas = el('div', {
    class: 'flow-canvas', style: `width:${drawing.width}px;height:${drawing.height}px`,
  }, [edgeLayer(drawing), ...nodes]);
  const fitButton = el('button', { class: 'flow-fit', type: 'button', title: 'Show the whole graph' }, 'Fit');
  viewport = el('div', { class: 'flow-viewport' }, [canvas, fitButton]);

  // Refit only when the graph's shape changes, not on every live update.
  if (shownProject !== state.projectId) views.delete(state.projectId);
  shownProject = state.projectId;
  const signature = tasks.map((t) => t.number + '@' + t.depth).join(',');
  const fit = enablePanZoom(viewport, canvas, state.projectId, drawing.width, drawing.height, signature);
  fitButton.addEventListener('click', fit);
  return el('div', { class: 'flow' }, [issueBanner(issues), viewport]);
}
