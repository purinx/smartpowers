(function () {
  'use strict';

  var COLUMNS = [
    { key: 'planned', label: 'Planned' },
    { key: 'in_progress', label: 'In Progress' },
    { key: 'blocked', label: 'Blocked' },
    { key: 'done', label: 'Done' },
    { key: 'other', label: 'Other' },
  ];

  var state = {
    repo: '',
    projects: [],
    project: null,
    projectId: null,
    taskNumber: null,
    view: readPref('view') || 'board',
    filter: '',
    previous: {},
  };

  var $ = function (id) { return document.getElementById(id); };

  function readPref(key) {
    try { return localStorage.getItem('smartpowers.' + key); } catch (_) { return null; }
  }
  function writePref(key, value) {
    try { localStorage.setItem('smartpowers.' + key, value); } catch (_) {}
  }

  function el(tag, attrs, children) {
    var node = document.createElement(tag);
    Object.keys(attrs || {}).forEach(function (name) {
      if (name === 'class') node.className = attrs[name];
      else if (name === 'html') node.innerHTML = attrs[name];
      else if (name.slice(0, 2) === 'on') node.addEventListener(name.slice(2), attrs[name]);
      else node.setAttribute(name, attrs[name]);
    });
    [].concat(children || []).forEach(function (child) {
      if (child == null || child === false) return;
      node.appendChild(typeof child === 'string' ? document.createTextNode(child) : child);
    });
    return node;
  }

  function escapeHtml(value) {
    return String(value).replace(/[&<>"']/g, function (c) {
      return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c];
    });
  }

  function statusIcon(key) {
    var svg = {
      planned: '<circle cx="7" cy="7" r="5.5" fill="none" stroke="var(--planned)" stroke-width="1.5" stroke-dasharray="2.2 1.6"/>',
      in_progress: '<circle cx="7" cy="7" r="5.5" fill="none" stroke="var(--progress)" stroke-width="1.5"/><path d="M7 3.5a3.5 3.5 0 0 1 0 7z" fill="var(--progress)"/>',
      blocked: '<circle cx="7" cy="7" r="6" fill="var(--blocked)"/><path d="M4.5 7h5" stroke="#fff" stroke-width="1.6" stroke-linecap="round"/>',
      done: '<circle cx="7" cy="7" r="6" fill="var(--done)"/><path d="M4.4 7.2l1.8 1.8 3.4-3.6" stroke="#fff" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/>',
      other: '<circle cx="7" cy="7" r="5.5" fill="none" stroke="var(--other)" stroke-width="1.5"/>',
    }[key] || '';
    var span = el('span', { class: 'status-icon', 'aria-hidden': 'true' });
    span.innerHTML = '<svg viewBox="0 0 14 14" width="14" height="14">' + svg + '</svg>';
    return span;
  }

  function projectStatusKey(status) {
    var s = String(status || '').toLowerCase();
    if (s === 'completed' || s === 'done') return 'done';
    if (s === 'blocked') return 'blocked';
    if (s === 'active' || s === 'in progress') return 'in_progress';
    return 'planned';
  }

  function relativeTime(iso) {
    var diff = (Date.now() - new Date(iso).getTime()) / 1000;
    if (diff < 60) return 'just now';
    if (diff < 3600) return Math.floor(diff / 60) + 'm ago';
    if (diff < 86400) return Math.floor(diff / 3600) + 'h ago';
    if (diff < 86400 * 30) return Math.floor(diff / 86400) + 'd ago';
    return new Date(iso).toLocaleDateString();
  }

  // ---- Minimal, escaping Markdown renderer --------------------------------

  function inline(text) {
    var codes = [];
    var out = escapeHtml(text).replace(/`([^`]+)`/g, function (_, code) {
      codes.push('<code>' + code + '</code>');
      return '\u0000' + (codes.length - 1) + '\u0000';
    });
    out = out
      .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
      .replace(/(^|[^*])\*([^*]+)\*/g, '$1<em>$2</em>')
      .replace(/\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>')
      .replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, '<span class="muted">$1</span>');
    return out.replace(/\u0000(\d+)\u0000/g, function (_, i) { return codes[+i]; });
  }

  function renderMarkdown(source) {
    var lines = source.split('\n');
    var html = [];
    var list = null;
    var para = [];
    var i = 0;

    function flushPara() {
      if (para.length) html.push('<p>' + inline(para.join(' ')) + '</p>');
      para = [];
    }
    function closeList() {
      if (list) html.push('</' + list + '>');
      list = null;
    }

    while (i < lines.length) {
      var line = lines[i];
      var m;
      if (/^```/.test(line)) {
        flushPara(); closeList();
        var code = [];
        i++;
        while (i < lines.length && !/^```/.test(lines[i])) code.push(lines[i++]);
        html.push('<pre><code>' + escapeHtml(code.join('\n')) + '</code></pre>');
      } else if ((m = /^(#{1,4})\s+(.*)$/.exec(line))) {
        flushPara(); closeList();
        html.push('<h' + m[1].length + '>' + inline(m[2]) + '</h' + m[1].length + '>');
      } else if (/^\|.*\|\s*$/.test(line)) {
        flushPara(); closeList();
        var rows = [];
        while (i < lines.length && /^\|.*\|\s*$/.test(lines[i])) rows.push(lines[i++]);
        i--;
        var cells = function (row) { return row.trim().slice(1, -1).split('|').map(function (c) { return c.trim(); }); };
        var body = rows.filter(function (row) { return !/^\|[\s\-:|]+\|$/.test(row.trim()); });
        html.push('<table>' + body.map(function (row, idx) {
          var tag = idx === 0 ? 'th' : 'td';
          return '<tr>' + cells(row).map(function (c) { return '<' + tag + '>' + inline(c) + '</' + tag + '>'; }).join('') + '</tr>';
        }).join('') + '</table>');
      } else if ((m = /^\s*([-*]|\d+\.)\s+(.*)$/.exec(line))) {
        flushPara();
        var type = /\d/.test(m[1]) ? 'ol' : 'ul';
        if (list !== type) { closeList(); html.push('<' + type + '>'); list = type; }
        html.push('<li>' + inline(m[2].replace(/^\[( |x)\]\s*/i, function (_, x) { return x === ' ' ? '☐ ' : '☑ '; })) + '</li>');
      } else if (!line.trim()) {
        flushPara(); closeList();
      } else {
        closeList();
        para.push(line.trim());
      }
      i++;
    }
    flushPara(); closeList();
    return html.join('\n');
  }

  function taskBody(markdown) {
    // Header fields are shown as properties; render from the first H2 on.
    var index = markdown.search(/^## /m);
    return index === -1 ? '' : markdown.slice(index);
  }

  // ---- Data ---------------------------------------------------------------

  function fetchJSON(url) {
    return fetch(url, { cache: 'no-store' }).then(function (res) {
      if (!res.ok) throw new Error(res.status + ' ' + url);
      return res.json();
    });
  }

  function load() {
    return fetchJSON('/api/projects').then(function (data) {
      state.repo = data.repo;
      state.projects = data.projects;
      if (!state.projectId || !state.projects.some(function (p) { return p.id === state.projectId; })) {
        state.projectId = state.projects.length ? state.projects[0].id : null;
      }
      if (!state.projectId) { state.project = null; return; }
      return fetchJSON('/api/projects/' + encodeURIComponent(state.projectId)).then(function (project) {
        state.project = project;
      });
    }).then(render);
  }

  function parseHash() {
    var parts = location.hash.replace(/^#\/?/, '').split('/').filter(Boolean);
    state.projectId = parts[0] ? decodeURIComponent(parts[0]) : state.projectId;
    state.taskNumber = parts[1] || null;
  }

  function navigate(projectId, taskNumber) {
    var hash = '#/' + encodeURIComponent(projectId) + (taskNumber ? '/' + taskNumber : '');
    if (location.hash !== hash) location.hash = hash;
  }

  // ---- Rendering ----------------------------------------------------------

  function visibleTasks() {
    var tasks = state.project ? state.project.tasks : [];
    var q = state.filter.trim().toLowerCase();
    if (!q) return tasks;
    return tasks.filter(function (t) {
      return [t.number, t.title, t.status, t.parallelGroup, (t.remote && t.remote.key) || '']
        .concat(t.requirements).join(' ').toLowerCase().indexOf(q) !== -1;
    });
  }

  function taskByNumber(number) {
    return (state.project ? state.project.tasks : []).filter(function (t) { return t.number === number; })[0];
  }

  function waitingOn(task) {
    if (task.statusKey === 'done') return [];
    return task.dependsOn.filter(function (n) {
      var dep = taskByNumber(n);
      return dep && dep.statusKey !== 'done';
    });
  }

  function chips(task, compact) {
    var nodes = [];
    var waiting = waitingOn(task);
    if (waiting.length) nodes.push(el('span', { class: 'chip warn', title: 'Waiting on unfinished dependencies' }, '⧗ ' + waiting.join(', ')));
    if (task.parallelGroup) nodes.push(el('span', { class: 'chip', title: 'Parallel group' }, '◇ ' + task.parallelGroup));
    var reqs = compact ? task.requirements.slice(0, 2) : task.requirements;
    reqs.forEach(function (r) { nodes.push(el('span', { class: 'chip' }, r)); });
    if (compact && task.requirements.length > 2) nodes.push(el('span', { class: 'chip' }, '+' + (task.requirements.length - 2)));
    return nodes;
  }

  function remoteLink(task) {
    if (!task.remote) return null;
    var label = task.remote.key || 'Remote';
    if (!task.remote.url) return el('span', { class: 'remote-key' }, label);
    return el('a', {
      class: 'remote-key', href: task.remote.url, target: '_blank', rel: 'noopener',
      onclick: function (e) { e.stopPropagation(); },
    }, label);
  }

  function changed(task) {
    var prev = state.previous[state.projectId + '/' + task.number];
    return prev && prev !== task.statusKey + '|' + task.updated;
  }

  function card(task) {
    var node = el('div', {
      class: 'card' + (task.number === state.taskNumber ? ' selected' : '') + (changed(task) ? ' flash' : ''),
      tabindex: '0',
      onclick: function () { navigate(state.projectId, task.number); },
      onkeydown: function (e) { if (e.key === 'Enter') navigate(state.projectId, task.number); },
    }, [
      el('div', { class: 'card-top' }, [
        el('span', { class: 'mono' }, task.number),
        remoteLink(task),
      ]),
      el('div', { class: 'card-title' }, [statusIcon(task.statusKey), el('span', null, task.title)]),
    ]);
    var meta = chips(task, true);
    if (meta.length) node.appendChild(el('div', { class: 'card-meta' }, meta));
    return node;
  }

  function columnsFor(tasks) {
    return COLUMNS.filter(function (c) {
      return c.key !== 'other' || tasks.some(function (t) { return t.statusKey === 'other'; });
    });
  }

  function renderBoard(tasks) {
    return el('div', { class: 'board' }, columnsFor(tasks).map(function (column) {
      var items = tasks.filter(function (t) { return t.statusKey === column.key; });
      return el('div', { class: 'column' }, [
        el('div', { class: 'column-head' }, [
          statusIcon(column.key), el('span', null, column.label),
          el('span', { class: 'column-count' }, String(items.length)),
        ]),
        el('div', { class: 'cards' }, items.length ? items.map(card) : el('div', { class: 'column-empty' }, 'No tasks')),
      ]);
    }));
  }

  function renderList(tasks) {
    return el('div', null, columnsFor(tasks).map(function (column) {
      var items = tasks.filter(function (t) { return t.statusKey === column.key; });
      if (!items.length) return null;
      return el('div', { class: 'group' }, [
        el('div', { class: 'group-head' }, [
          statusIcon(column.key), el('span', null, column.label),
          el('span', { class: 'column-count' }, String(items.length)),
        ]),
      ].concat(items.map(function (task) {
        return el('div', {
          class: 'row' + (task.number === state.taskNumber ? ' selected' : '') + (changed(task) ? ' flash' : ''),
          onclick: function () { navigate(state.projectId, task.number); },
        }, [
          statusIcon(task.statusKey),
          el('span', { class: 'mono muted' }, task.number),
          el('span', { class: 'row-title' }, task.title),
          el('span', { class: 'row-chips' }, chips(task, true).concat([remoteLink(task)])),
          el('span', { class: 'row-date', title: task.updated }, relativeTime(task.updated)),
        ]);
      })));
    }));
  }

  function renderNav() {
    $('repo-name').textContent = state.repo || 'Smartpowers';
    var nav = $('project-nav');
    nav.textContent = '';
    if (!state.projects.length) {
      nav.appendChild(el('div', { class: 'nav-empty' }, 'No projects yet'));
      return;
    }
    state.projects.forEach(function (p) {
      var done = p.counts.done || 0;
      nav.appendChild(el('button', {
        class: 'nav-item' + (p.id === state.projectId ? ' active' : ''),
        title: p.id,
        onclick: function () { navigate(p.id); },
      }, [
        statusIcon(projectStatusKey(p.status)),
        el('span', { class: 'nav-title' }, p.title),
        el('span', { class: 'nav-count' }, p.taskCount ? done + '/' + p.taskCount : ''),
      ]));
    });
  }

  function renderHeader() {
    var project = state.project;
    $('project-header').hidden = !project;
    $('crumb-title').textContent = project ? project.title : '—';
    document.title = project ? project.title + ' · Smartpowers' : 'Smartpowers Board';
    if (!project) return;
    $('project-title').textContent = project.title;
    $('project-status').textContent = project.status;
    $('project-id').textContent = project.id;

    var phases = $('phases');
    phases.textContent = '';
    project.phases.forEach(function (phase) {
      var status = phase.status || 'Missing';
      var cls = /^approved$/i.test(status) ? 'approved' : /^draft$/i.test(status) ? 'draft' : '';
      phases.appendChild(el('span', { class: 'phase ' + cls, title: phase.file + ': ' + status }, [
        el('span', { class: 'phase-dot' }), phase.name,
      ]));
    });

    var total = project.taskCount;
    var done = project.counts.done || 0;
    var pct = total ? Math.round((done / total) * 100) : 0;
    $('progress-fill').style.width = pct + '%';
    $('progress-text').textContent = total ? done + ' / ' + total + ' done · ' + pct + '%' : 'No tasks yet';
  }

  function renderContent() {
    var content = $('content');
    content.textContent = '';
    if (!state.projects.length) {
      content.appendChild(el('div', { class: 'empty' }, [
        el('h3', null, 'No Smartpowers projects'),
        el('div', null, 'Projects appear here once docs/smartpowers/projects/<id>/ exists.'),
      ]));
      return;
    }
    if (!state.project) return;
    if (!state.project.tasks.length) {
      content.appendChild(el('div', { class: 'empty' }, [
        el('h3', null, 'No tasks yet'),
        el('div', null, 'Tasks appear after the implementation plan creates tasks/NN_*.md files.'),
      ]));
      return;
    }
    var tasks = visibleTasks();
    content.appendChild(state.view === 'list' ? renderList(tasks) : renderBoard(tasks));
  }

  function prop(label, value) {
    return [el('dt', null, label), el('dd', null, value)];
  }

  function renderPanel() {
    var task = state.taskNumber && taskByNumber(state.taskNumber);
    $('panel').hidden = !task;
    if (!task) return;
    $('panel-id').textContent = state.project.id + ' / ' + task.number;
    $('panel-title').textContent = task.title;

    var props = $('panel-props');
    props.textContent = '';
    var deps = task.dependsOn.length ? task.dependsOn.map(function (n) {
      var dep = taskByNumber(n);
      return el('span', {
        class: 'chip', style: 'cursor:pointer',
        onclick: function () { navigate(state.projectId, n); },
      }, [dep ? statusIcon(dep.statusKey) : null, n + (dep ? ' ' + dep.title : '')]);
    }) : 'None';
    var remote = task.remote
      ? (task.remote.url ? el('a', { href: task.remote.url, target: '_blank', rel: 'noopener' }, task.remote.key || task.remote.url) : task.remote.raw)
      : el('span', { class: 'muted' }, 'Not linked');
    [
      prop('Status', [statusIcon(task.statusKey), task.status]),
      prop('Requirements', task.requirements.length ? task.requirements.map(function (r) { return el('span', { class: 'chip' }, r); }) : '—'),
      prop('Depends on', deps),
      prop('Parallel group', task.parallelGroup || '—'),
      prop('Remote task', remote),
      prop('File', el('code', { class: 'mono' }, state.project.path + '/tasks/' + task.file)),
      prop('Updated', relativeTime(task.updated)),
    ].forEach(function (pair) { pair.forEach(function (n) { props.appendChild(n); }); });

    $('panel-body').innerHTML = renderMarkdown(taskBody(task.body));
  }

  function render() {
    document.querySelectorAll('.segmented button').forEach(function (b) {
      b.classList.toggle('active', b.getAttribute('data-view') === state.view);
    });
    renderNav();
    renderHeader();
    renderContent();
    renderPanel();
    rememberStatuses();
  }

  function rememberStatuses() {
    state.previous = {};
    (state.project ? state.project.tasks : []).forEach(function (t) {
      state.previous[state.projectId + '/' + t.number] = t.statusKey + '|' + t.updated;
    });
  }

  // ---- Live updates -------------------------------------------------------

  function connect() {
    var live = $('live');
    var text = $('live-text');
    var source = new EventSource('/api/events');
    source.addEventListener('ready', function () {
      live.className = 'live on'; text.textContent = 'Live';
    });
    source.addEventListener('change', function () { load(); });
    source.onerror = function () {
      live.className = 'live off'; text.textContent = 'Reconnecting…';
    };
    source.onopen = function () { load(); };
  }

  // ---- Events -------------------------------------------------------------

  document.querySelectorAll('.segmented button').forEach(function (b) {
    b.addEventListener('click', function () {
      state.view = b.getAttribute('data-view');
      writePref('view', state.view);
      render();
    });
  });

  $('filter').addEventListener('input', function (e) {
    state.filter = e.target.value;
    renderContent();
  });

  $('panel-close').addEventListener('click', function () { navigate(state.projectId); });

  window.addEventListener('hashchange', function () {
    var before = state.projectId;
    parseHash();
    if (before !== state.projectId) { state.previous = {}; load(); } else render();
  });

  document.addEventListener('keydown', function (e) {
    var typing = /^(INPUT|TEXTAREA)$/.test(document.activeElement.tagName);
    if (e.key === 'Escape') {
      if (typing) document.activeElement.blur();
      else if (state.taskNumber) navigate(state.projectId);
      return;
    }
    if (typing || e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.key === '/') { e.preventDefault(); $('filter').focus(); }
    else if (e.key === 'b' || e.key === 'l') {
      state.view = e.key === 'b' ? 'board' : 'list';
      writePref('view', state.view);
      render();
    }
  });

  parseHash();
  load().catch(function (err) { console.error(err); });
  connect();
})();
