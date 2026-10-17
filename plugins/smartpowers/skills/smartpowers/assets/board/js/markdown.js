// Minimal Markdown renderer for task files. Input is escaped first, so the
// output only contains tags generated here. Supports headings, paragraphs,
// lists (with checkboxes), fenced code, tables, inline code, bold, italic,
// and absolute links. Relative links render as plain text.

import { escapeHtml } from './dom.js';

const PLACEHOLDER = '\u0000';

function inline(text) {
  // Protect code spans from emphasis/link rules, then restore them.
  const codes = [];
  let html = escapeHtml(text).replace(/`([^`]+)`/g, (_, code) => {
    codes.push('<code>' + code + '</code>');
    return PLACEHOLDER + (codes.length - 1) + PLACEHOLDER;
  });
  html = html
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/(^|[^*])\*([^*]+)\*/g, '$1<em>$2</em>')
    .replace(/\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>')
    .replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, '<span class="muted">$1</span>');
  return html.replace(new RegExp(PLACEHOLDER + '(\\d+)' + PLACEHOLDER, 'g'), (_, i) => codes[+i]);
}

const TABLE_ROW = /^\|.*\|\s*$/;
const TABLE_DIVIDER = /^\|[\s\-:|]+\|$/;

function renderTable(rows) {
  const cells = (row) => row.trim().slice(1, -1).split('|').map((cell) => cell.trim());
  const body = rows.filter((row) => !TABLE_DIVIDER.test(row.trim()));
  const html = body.map((row, index) => {
    const tag = index === 0 ? 'th' : 'td';
    return '<tr>' + cells(row).map((cell) => `<${tag}>${inline(cell)}</${tag}>`).join('') + '</tr>';
  });
  return '<table>' + html.join('') + '</table>';
}

function listItem(text) {
  const withBox = text.replace(/^\[( |x)\]\s*/i, (_, mark) => (mark === ' ' ? '☐ ' : '☑ '));
  return '<li>' + inline(withBox) + '</li>';
}

export function renderMarkdown(source) {
  const lines = source.split('\n');
  const html = [];
  let paragraph = [];
  let list = null; // 'ul' | 'ol' | null

  const flushParagraph = () => {
    if (paragraph.length) html.push('<p>' + inline(paragraph.join(' ')) + '</p>');
    paragraph = [];
  };
  const closeList = () => {
    if (list) html.push('</' + list + '>');
    list = null;
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    let match;

    if (/^```/.test(line)) {
      flushParagraph(); closeList();
      const code = [];
      for (i++; i < lines.length && !/^```/.test(lines[i]); i++) code.push(lines[i]);
      html.push('<pre><code>' + escapeHtml(code.join('\n')) + '</code></pre>');
    } else if ((match = /^(#{1,4})\s+(.*)$/.exec(line))) {
      flushParagraph(); closeList();
      const level = match[1].length;
      html.push(`<h${level}>${inline(match[2])}</h${level}>`);
    } else if (TABLE_ROW.test(line)) {
      flushParagraph(); closeList();
      const rows = [];
      for (; i < lines.length && TABLE_ROW.test(lines[i]); i++) rows.push(lines[i]);
      i--;
      html.push(renderTable(rows));
    } else if ((match = /^\s*([-*]|\d+\.)\s+(.*)$/.exec(line))) {
      flushParagraph();
      const type = /\d/.test(match[1]) ? 'ol' : 'ul';
      if (list !== type) { closeList(); html.push('<' + type + '>'); list = type; }
      html.push(listItem(match[2]));
    } else if (!line.trim()) {
      flushParagraph(); closeList();
    } else {
      closeList();
      paragraph.push(line.trim());
    }
  }
  flushParagraph(); closeList();
  return html.join('\n');
}

// Header fields are shown as panel properties; render the body from the first H2.
export function taskBody(markdown) {
  const index = markdown.search(/^## /m);
  return index === -1 ? '' : markdown.slice(index);
}
