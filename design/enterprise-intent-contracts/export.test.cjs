const assert = require('node:assert/strict');
const { test } = require('node:test');
const { mkdtemp, copyFile, rm } = require('node:fs/promises');
const { tmpdir } = require('node:os');
const { join } = require('node:path');
const { pathToFileURL } = require('node:url');
const { chromium } = require('playwright');

test('one exported file runs offline with all five projects and both comparisons', async () => {
  const folder = await mkdtemp(join(tmpdir(), 'intent-contracts-export-'));
  const browser = await chromium.launch();
  try {
    const file = join(folder, 'shared.html');
    await copyFile(join(__dirname, 'enterprise-intent-contracts.html'), file);
    const context = await browser.newContext({ offline: true, reducedMotion: 'reduce' });
    const page = await context.newPage();
    const errors = [], remote = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('request', request => { if (/^https?:/.test(request.url())) remote.push(request.url()); });
    await page.goto(pathToFileURL(file).href);
    await page.evaluate(() => document.fonts.ready);
    for (const id of ['modeling-before', 'modeling-after', 'before', 'after']) {
      assert.ok(await page.locator(`#${id} [data-node]`).count(), `${id} did not render`);
    }
    for (const id of ['projects-p2', 'projects-p4', 'projects-p5']) {
      await page.locator(`#${id} > svg`).waitFor();
      assert.ok(await page.locator(`#${id} svg text`).count(), `${id} did not render`);
    }
    await page.locator('#modeling-run').click();
    await page.locator('#modeling-after-result[data-outcome="pending"]').waitFor();
    await page.locator('#evidence').selectOption('rejected');
    await page.locator('#run').click();
    await page.locator('#after-result[data-outcome="blocked"]').waitFor();
    await page.locator('#evidence').selectOption('approved');
    await page.locator('#run').click();
    await page.locator('#after-result[data-outcome="allowed"]').waitFor();
    for (const width of [320, 1280]) {
      await page.setViewportSize({ width, height: 900 });
      await page.waitForFunction(() => document.documentElement.scrollWidth === window.innerWidth);
    }
    assert.equal(await page.evaluate(() => document.fonts.check('16px "IBM Plex Sans"')), true);
    assert.equal(await page.locator('script[src], link[rel="stylesheet"]').count(), 0);
    assert.deepEqual(remote, []);
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await rm(folder, { recursive: true, force: true });
  }
});

test('enforcement detail export runs offline and blocks every failure scenario', async () => {
  const folder = await mkdtemp(join(tmpdir(), 'policy-detail-export-'));
  const browser = await chromium.launch();
  try {
    const file = join(folder, 'enforcement.html');
    await copyFile(join(__dirname, 'policy-enforcement.html'), file);
    const context = await browser.newContext({ offline: true, reducedMotion: 'reduce' });
    const page = await context.newPage();
    const errors = [], remote = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('request', request => { if (/^https?:/.test(request.url())) remote.push(request.url()); });
    await page.goto(pathToFileURL(file).href);
    await page.locator('#trace-map [data-station]').first().waitFor();
    for (const scenario of ['allowed', 'denied', 'stale', 'jwt', 'endpoint', 'unavailable']) {
      await page.locator('#trace-scenario').selectOption(scenario);
      await page.locator('#trace-run').click();
      const outcome = scenario === 'allowed' ? 'allowed' : 'blocked';
      await page.locator(`#trace-result[data-outcome="${outcome}"]`).waitFor();
      await page.waitForFunction(() => !document.querySelector('#trace-run').disabled);
    }
    await page.setViewportSize({ width: 320, height: 900 });
    await page.waitForFunction(() => document.documentElement.scrollWidth === window.innerWidth);
    await page.locator('.detail-notes summary').click();
    await page.locator('.detail-source-map').waitFor();
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth), 320);
    assert.equal(await page.locator('script[src], link[rel="stylesheet"]').count(), 0);
    assert.deepEqual(remote, []);
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await rm(folder, { recursive: true, force: true });
  }
});
