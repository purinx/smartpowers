// Flow view: the task dependency graph, one column per depth.
//
// The server computes `depth`, `ready`, and `graphIssues`; this module only
// lays tasks out and draws them. It never decides readiness itself.
//
// Layout: columns by depth, each in plan order (the API's task order), then
// one left-to-right barycenter pass that sorts a column by the mean row of
// each task's dependencies in earlier columns. Ties keep plan order. Nodes
// are fixed-size HTML cards placed absolutely; one inline <svg> beneath them
// draws the edges.

import { el } from '../dom.js';
import { openTask } from '../router.js';
import { statusIcon } from '../status.js';
import { taskClasses } from './task-parts.js';

const NODE_WIDTH = 232;
const NODE_HEIGHT = 76;
const COLUMN_GAP = 72;
const ROW_GAP = 16;
const PADDING = 8;
const SVG_NS = 'http://www.w3.org/2000/svg';

// Columns of tasks, left to right, each ordered top to bottom.
function layoutColumns(tasks) {
  const byDepth = new Map();
  for (const task of tasks) {
    const depth = task.depth ?? 0;
    if (!byDepth.has(depth)) byDepth.set(depth, []);
    byDepth.get(depth).push(task);
  }
  const columns = [...byDepth.keys()].sort((a, b) => a - b).map((depth) => byDepth.get(depth));

  const row = new Map(); // task number -> row index in its (already placed) column
  columns.forEach((column, index) => {
    if (index > 0) {
      const keyed = column.map((task, order) => {
        const rows = task.dependsOn.filter((number) => row.has(number)).map((number) => row.get(number));
        const key = rows.length ? rows.reduce((sum, r) => sum + r, 0) / rows.length : order;
        return { task, key, order };
      });
      keyed.sort((a, b) => a.key - b.key || a.order - b.order);
      column.splice(0, column.length, ...keyed.map((entry) => entry.task));
    }
    column.forEach((task, r) => row.set(task.number, r));
  });
  return columns;
}

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

// Vertical center of row `row`, and of the gap just above it.
const rowCenter = (row) => PADDING + row * (NODE_HEIGHT + ROW_GAP) + NODE_HEIGHT / 2;
const gapAbove = (row) => PADDING + row * (NODE_HEIGHT + ROW_GAP) - ROW_GAP / 2;

// The y of a horizontal lane that an edge skipping columns `skipped` (their
// node counts) can follow without passing under a node: the source's own row
// when those columns are empty there, or else the row gap, or the space below
// the tallest skipped column, nearest to both ends.
function laneY(fromRow, toRow, skipped) {
  const tallest = Math.max(...skipped);
  if (fromRow === toRow && fromRow >= tallest) return rowCenter(fromRow);
  const target = (rowCenter(fromRow) + rowCenter(toRow)) / 2;
  let best = gapAbove(1);
  for (let row = 1; row <= tallest; row += 1) {
    if (Math.abs(gapAbove(row) - target) < Math.abs(best - target)) best = gapAbove(row);
  }
  return best;
}

// Edges from the right middle of each dependency to the left middle of its
// dependent. Dependencies that are not on the canvas get no edge. Returns the
// edge paths and the lowest y they reach.
function edgePaths(tasks, positions, columns) {
  const byNumber = new Map(tasks.map((task) => [task.number, task]));
  const paths = [];
  let bottom = 0;
  for (const task of tasks) {
    const to = positions.get(task.number);
    for (const number of new Set(task.dependsOn)) {
      const from = positions.get(number);
      if (!from) continue;
      const done = byNumber.get(number).statusKey === 'done';
      const x1 = from.x + NODE_WIDTH;
      const y1 = from.y + NODE_HEIGHT / 2;
      const y2 = to.y + NODE_HEIGHT / 2;
      let d;
      if (to.column > from.column + 1) {
        // A straight curve would pass under the nodes of the skipped columns
        // and look like it ends at one of them, so follow a free lane.
        const skipped = columns.slice(from.column + 1, to.column).map((column) => column.length);
        const lane = laneY(from.row, to.row, skipped);
        const xa = x1 + COLUMN_GAP;
        const xb = to.x - COLUMN_GAP;
        const x2 = to.x - 1;
        const bend = COLUMN_GAP / 2;
        d = `M${x1} ${y1}C${x1 + bend} ${y1} ${xa - bend} ${lane} ${xa} ${lane}`
          + `L${xb} ${lane}C${xb + bend} ${lane} ${x2 - bend} ${y2} ${x2} ${y2}`;
        bottom = Math.max(bottom, lane);
      } else if (to.x > from.x) {
        const x2 = to.x - 1;
        const bend = Math.max(24, (x2 - x1) / 2);
        d = `M${x1} ${y1}C${x1 + bend} ${y1} ${x2 - bend} ${y2} ${x2} ${y2}`;
      } else {
        // Only a cycle puts a dependency in the same or a later column: loop
        // out to the right and come back into the dependent's right side.
        const x2 = to.x + NODE_WIDTH + 1;
        const bend = Math.min(COLUMN_GAP - 8, 24 + Math.abs(y2 - y1) / 4) + (y2 < y1 ? 16 : 0);
        d = `M${x1} ${y1}C${x1 + bend} ${y1} ${x2 + bend} ${y2} ${x2} ${y2}`;
      }
      paths.push(svgEl('path', {
        class: 'flow-edge ' + (done ? 'done' : 'pending'),
        d,
        'marker-end': done ? 'url(#flow-arrow-done)' : 'url(#flow-arrow-pending)',
      }));
    }
  }
  return { paths, bottom };
}

function edgeLayer(paths, width, height) {
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

function node(task, position) {
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
  return el('div', {
    class: taskClasses('flow-node', task),
    style: `left:${position.x}px;top:${position.y}px`,
    tabindex: '0',
    role: 'button',
    title: task.number + ' ' + task.title,
    onclick: () => openTask(task.number),
    onkeydown: (event) => { if (event.key === 'Enter') openTask(task.number); },
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

export function renderFlow(tasks, issues) {
  const columns = layoutColumns(tasks);
  const positions = new Map();
  const nodes = [];
  columns.forEach((column, c) => {
    column.forEach((task, r) => {
      const position = {
        x: PADDING + c * (NODE_WIDTH + COLUMN_GAP),
        y: PADDING + r * (NODE_HEIGHT + ROW_GAP),
        column: c,
        row: r,
      };
      positions.set(task.number, position);
      nodes.push(node(task, position)); // DOM order = column order, so Tab follows columns
    });
  });
  const rows = Math.max(0, ...columns.map((column) => column.length));
  // A cycle may loop edges out past the last column, so leave room for them.
  const width = PADDING * 2 + columns.length * (NODE_WIDTH + COLUMN_GAP) - (columns.length && !issues.length ? COLUMN_GAP : 0);
  const { paths, bottom } = edgePaths(tasks, positions, columns);
  // A lane below the tallest column needs room under the last row.
  const height = Math.max(PADDING * 2 + rows * (NODE_HEIGHT + ROW_GAP) - (rows ? ROW_GAP : 0), bottom + PADDING);

  const canvas = el('div', { class: 'flow-canvas', style: `width:${width}px;height:${height}px` }, [
    edgeLayer(paths, width, height),
    ...nodes,
  ]);
  return el('div', { class: 'flow' }, [
    issueBanner(issues),
    tasks.length ? el('div', { class: 'flow-scroll' }, canvas) : el('div', { class: 'column-empty' }, 'No matching tasks'),
  ]);
}
