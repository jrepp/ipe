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
