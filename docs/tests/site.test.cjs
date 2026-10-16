const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { JSDOM } = require('jsdom');
const root = path.resolve(__dirname, '..');
const data = JSON.parse(fs.readFileSync(path.join(root, 'data/site-data.json'), 'utf8'));

async function load(page, script, failure = false) {
  const dom = new JSDOM(fs.readFileSync(path.join(root, page), 'utf8'), {
    url: `http://localhost/docs/${page}`, runScripts: 'outside-only',
  });
  dom.window.fetch = async () => ({ ok: !failure, json: async () => structuredClone(data) });
  dom.window.eval(fs.readFileSync(path.join(root, script), 'utf8'));
  await new Promise(resolve => setImmediate(resolve));
  return dom;
}

test('AST starts expanded, selects nodes, switches examples, collapses and expands', async () => {
  const dom = await load('ast.html', 'ast.js');
  try {
    const { document, Event } = dom.window;
    assert.match(document.querySelector('#ast-tree').textContent, /RequireApproval/);
    assert.ok(document.querySelectorAll('#ast-tree details[open]').length > 5);
    const pathNode = document.querySelector('[data-kind="Path"]');
    pathNode.click();
    assert.match(document.querySelector('#ast-detail').textContent, /"kind": "Path"/);
    document.querySelector('#ast-collapse').click();
    assert.equal(document.querySelectorAll('#ast-tree details[open]').length, 0);
    document.querySelector('#ast-expand').click();
    assert.ok(document.querySelectorAll('#ast-tree details[open]').length > 5);
    const selector = document.querySelector('#ast-example');
    selector.value = '2'; selector.dispatchEvent(new Event('change'));
    assert.match(document.querySelector('#ast-source').textContent, /BlockProduction/);
    assert.match(document.querySelector('#ast-tree').textContent, /Denies/);
    assert.doesNotMatch(document.querySelector('#ast-tree').textContent, /RequireApproval/);
  } finally { dom.window.close(); }
});

test('overview renders a tree without depending on the Mermaid CDN', async () => {
  const dom = await load('index.html', 'ast.js');
  try {
    assert.ok(dom.window.document.querySelectorAll('#ast-tree .ast-node').length > 10);
    assert.equal(dom.window.mermaid, undefined);
  } finally { dom.window.close(); }
});

test('dashboard controls update real data, comparison bars and percentile curve', async () => {
  const dom = await load('performance.html', 'performance.js');
  try {
    const { document, Event } = dom.window;
    assert.equal(document.querySelector('#perf-content').hidden, false);
    assert.equal(document.querySelectorAll('#latency-chart rect').length, 3);
    assert.equal(document.querySelectorAll('#distribution-chart circle').length, 100);
    assert.equal(document.querySelectorAll('#measurements tr').length, 3);
    const workload = document.querySelector('#workload');
    workload.value = '2'; workload.dispatchEvent(new Event('change'));
    const percentile = document.querySelector('#percentile');
    percentile.value = 'p50_us'; percentile.dispatchEvent(new Event('change'));
    assert.equal(document.querySelector('#latency').textContent, data.workloads[2].p50_us.toLocaleString(undefined, { maximumFractionDigits: 3 }));
    assert.match(document.querySelector('tr.selected').textContent, /32 comparisons/);
    assert.equal(document.querySelector('#latency-label').textContent, 'p50 latency');
    assert.match(document.querySelector('#perf-status').textContent, new RegExp(data.revision));
    assert.equal(document.querySelector('a[download]').getAttribute('href'), 'data/site-data.json');
  } finally { dom.window.close(); }
});

test('missing data produces actionable errors instead of fabricated measurements', async () => {
  for (const [page, script, selector] of [['ast.html', 'ast.js', '#ast-tree'], ['performance.html', 'performance.js', '#perf-status']]) {
    const dom = await load(page, script, true);
    try {
      assert.match(dom.window.document.querySelector(selector).textContent, /make docs-site/);
      if (page === 'performance.html') assert.equal(dom.window.document.querySelector('#perf-content').hidden, true);
    } finally { dom.window.close(); }
  }
});

test('interior pages share one sidebar and mark only themselves as current', () => {
  const pages = ['decision.html', 'status.html', 'contract.html', 'ast.html', 'performance.html', 'doc.html'];
  const sidebars = pages.map(page => {
    const { document } = new JSDOM(fs.readFileSync(path.join(root, page), 'utf8')).window;
    const current = [...document.querySelectorAll('.sidebar [aria-current="page"]')].map(link => link.getAttribute('href'));
    // The Markdown viewer marks its current document at runtime.
    assert.deepEqual(current, page === 'doc.html' ? [] : [page]);
    return [...document.querySelectorAll('.sidebar a')].map(link => `${link.getAttribute('href')}|${link.textContent}`);
  });
  for (const sidebar of sidebars.slice(1)) assert.deepEqual(sidebar, sidebars[0]);
});

test('journey chips and steps point at journeys and pages that exist', () => {
  const { document } = new JSDOM(fs.readFileSync(path.join(root, 'index.html'), 'utf8')).window;
  const journeys = new Set([...document.querySelectorAll('.path')].map(path => path.id));
  assert.equal(journeys.size, 4);
  for (const page of ['decision.html', 'status.html', 'contract.html', 'ast.html', 'performance.html']) {
    const doc = new JSDOM(fs.readFileSync(path.join(root, page), 'utf8')).window.document;
    const chips = [...doc.querySelectorAll('.chip-path')].map(chip => chip.getAttribute('href').split('#')[1]);
    assert.ok(chips.length > 0, `${page} belongs to a journey`);
    chips.forEach(id => assert.ok(journeys.has(id), `${page} → ${id}`));
    const listed = [...document.querySelectorAll('.path a')].some(link => link.getAttribute('href') === page);
    assert.ok(listed, `${page} appears in a landing journey`);
  }
});

test('landing explains the selected node without the JSON detail panel', async () => {
  const dom = await load('index.html', 'ast.js');
  try {
    const { document } = dom.window;
    document.querySelector('[data-kind="Path"]').click();
    assert.match(document.querySelector('#ast-explanation').textContent, /^Path: A field reference/);
    assert.match(document.querySelector('#ast-source').textContent, /policy RequireApproval/);
  } finally { dom.window.close(); }
});

async function loadDoc(query, { markdown = '# Doc\n', marked = true, mermaid = true } = {}) {
  const dom = new JSDOM(fs.readFileSync(path.join(root, 'doc.html'), 'utf8'), {
    url: `http://localhost/ipe/docs/doc.html${query}`, runScripts: 'outside-only',
  });
  const { window } = dom;
  const requests = [];
  window.fetch = async url => { requests.push(String(url)); return { ok: true, text: async () => markdown }; };
  window.HTMLElement.prototype.scrollIntoView = () => {};
  if (marked) window.marked = { parse: text => text };  // Fixture markdown is already HTML.
  const diagrams = [];
  if (mermaid) window.mermaid = {
    initialize() {},
    render: async (id, text) => {
      if (text.includes('broken')) throw new Error('parse error');
      diagrams.push(text);
      return { svg: `<svg id="${id}"></svg>` };
    },
  };
  window.eval(fs.readFileSync(path.join(root, 'doc.js'), 'utf8'));
  await new Promise(resolve => setTimeout(resolve, 10));
  return { dom, requests, diagrams };
}

test('markdown viewer renders diagrams, keeps doc links in the viewer, and builds an outline', async () => {
  const markdown = '<h1>Architecture</h1><h2>Layers &amp; tiers</h2><p><a href="BYTECODE.md#opcodes">ops</a> '
    + '<a href="../crates/ipe-core/src/jit.rs">jit</a> <a href="https://example.com/x.md">ext</a></p>'
    + '<pre><code class="language-mermaid">graph TD; A--&gt;B</code></pre><h2>Storage</h2><pre><code class="language-rust">fn x() {}</code></pre>'
    + '<pre><code class="language-mermaid">graph TD; C--&gt;D</code></pre><pre><code class="language-mermaid">broken</code></pre>';
  const { dom, requests, diagrams } = await loadDoc('?path=ARCHITECTURE.md', { markdown });
  try {
    const { document } = dom.window;
    assert.deepEqual(requests, ['http://localhost/ipe/docs/ARCHITECTURE.md']);
    assert.deepEqual(diagrams, ['graph TD; A-->B', 'graph TD; C-->D']);
    assert.deepEqual([...document.querySelectorAll('#doc-body .mermaid svg')].map(svg => svg.id), ['doc-diagram-0', 'doc-diagram-1']);
    assert.equal([...document.querySelectorAll('#doc-body pre')].at(-1).textContent, 'broken');
    assert.equal(document.querySelectorAll('#doc-body code.language-rust').length, 1);
    const links = [...document.querySelectorAll('#doc-body a')].map(link => link.getAttribute('href'));
    assert.deepEqual(links, ['doc.html?path=BYTECODE.md#opcodes', 'http://localhost/ipe/crates/ipe-core/src/jit.rs', 'https://example.com/x.md']);
    assert.deepEqual([...document.querySelectorAll('#doc-toc a')].map(link => link.getAttribute('href')), ['#layers--tiers', '#storage']);
    assert.equal(document.title, 'Architecture · IPE');
    assert.equal(document.querySelector('.sidebar [aria-current="page"]').getAttribute('href'), 'doc.html?path=ARCHITECTURE.md');
    assert.match(document.querySelector('#doc-status').textContent, /3 diagrams, 1 shown as source \(invalid Mermaid\)$/);
  } finally { dom.window.close(); }
});

test('markdown viewer resolves documents outside docs/ and links back through the viewer', async () => {
  const { dom, requests } = await loadDoc('?path=../rfcs/INDEX.md', { markdown: '<a href="001-sidecar-service-architecture.md">rfc</a><a href="../docs/AST.md">ast</a>' });
  try {
    assert.deepEqual(requests, ['http://localhost/ipe/rfcs/INDEX.md']);
    const links = [...dom.window.document.querySelectorAll('#doc-body a')].map(link => link.getAttribute('href'));
    assert.deepEqual(links, ['doc.html?path=../rfcs/001-sidecar-service-architecture.md', 'doc.html?path=AST.md']);
  } finally { dom.window.close(); }
});

test('markdown viewer shows raw source when the renderer is unavailable and rejects other paths', async () => {
  const offline = await loadDoc('?path=AST.md', { markdown: '# AST\n```mermaid\ngraph TD\n```', marked: false, mermaid: false });
  try {
    assert.match(offline.dom.window.document.querySelector('#doc-body pre').textContent, /```mermaid/);
    assert.match(offline.dom.window.document.querySelector('#doc-status').textContent, /raw source/);
  } finally { offline.dom.window.close(); }
  for (const query of ['?path=https://evil.example/x.md', '?path=site.css', '?path=//evil.example/x.md']) {
    const rejected = await loadDoc(query);
    try {
      assert.deepEqual(rejected.requests, []);
      assert.match(rejected.dom.window.document.querySelector('#doc-status').textContent, /not a repository Markdown path/);
    } finally { rejected.dom.window.close(); }
  }
});

test('every viewer link names a tracked Markdown file', () => {
  for (const page of fs.readdirSync(root).filter(name => name.endsWith('.html'))) {
    const { document } = new JSDOM(fs.readFileSync(path.join(root, page), 'utf8')).window;
    for (const link of document.querySelectorAll('a[href^="doc.html?path="]')) {
      const target = new URLSearchParams(link.getAttribute('href').split('?')[1].split('#')[0]).get('path');
      assert.ok(fs.existsSync(path.join(root, target)), `${page} → ${target}`);
    }
  }
});
