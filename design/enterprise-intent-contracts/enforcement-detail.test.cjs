const assert = require('node:assert/strict');
const { after, before, test } = require('node:test');
const { pathToFileURL } = require('node:url');
const { chromium } = require('playwright');

let browser;
before(async () => { browser = await chromium.launch(); });
after(async () => { await browser?.close(); });

async function open(context, options) {
  const page = await browser.newPage(options);
  context.after(() => page.close());
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  context.after(() => assert.deepEqual(errors, []));
  await page.goto(new URL('./enforcement-detail.html', pathToFileURL(__filename)).href);
  await page.locator('#trace-map [data-station="engine"]').waitFor();
  return page;
}

test('only a usable allow reaches the action, and the service rejects invalid routes before evaluation', async context => {
  const page = await open(context, { reducedMotion: 'reduce' });
  const station = name => page.locator(`[data-station="${name}"]`);
  const cases = [
    ['allowed', 'allowed', 'Allow', true, true, true],
    ['denied', 'blocked', 'Deny', true, true, true],
    ['stale', 'blocked', 'Indeterminate', true, true, true],
    ['jwt', 'blocked', 'JWT rejected', false, false, false],
    ['endpoint', 'blocked', 'Route rejected', true, false, false],
    ['unavailable', 'blocked', 'No decision', true, true, false],
  ];
  for (const [scenario, outcome, decision, checkedRoute, calledEngine, captured] of cases) {
    await page.locator('#trace-scenario').selectOption(scenario);
    await page.locator('#trace-run').press('Enter');
    const result = page.locator(`#trace-result[data-outcome="${outcome}"]`);
    await result.waitFor();
    assert.equal(await result.getAttribute('data-decision'), decision);
    assert.equal((await station('registry').getAttribute('class')).includes('reached'), checkedRoute);
    assert.equal((await station('engine').getAttribute('class')).includes('reached'), calledEngine);
    assert.equal((await station('policy').getAttribute('class')).includes('reached'), captured);
    assert.equal((await station('facts').getAttribute('class')).includes('reached'), captured);
    assert.equal((await station('facts').getAttribute('class')).includes('blocked'), scenario === 'stale');
    assert.equal((await station('action').getAttribute('class')).includes('reached'), outcome === 'allowed');
    assert.equal((await station('gate').getAttribute('class')).includes('reached'), outcome === 'allowed');
    assert.equal(await page.locator('#trace-map .packet').count(), 0);
  }
});

test('a change cancels animation and a replay finishes without packets', async context => {
  const page = await open(context, { reducedMotion: 'no-preference' });
  const run = page.locator('#trace-run');
  await run.click();
  const packets = page.locator('#trace-map .packet');
  await packets.first().waitFor();
  await page.locator('#trace-scenario').selectOption('jwt');
  assert.equal(await packets.count(), 0);
  assert.equal(await page.locator('#trace-result').getAttribute('data-outcome'), null);
  await run.click();
  await page.locator('#trace-result[data-outcome="blocked"]').waitFor();
  assert.equal(await packets.count(), 0);
  assert.equal(await run.isEnabled(), true);
});

test('the detail layout fits narrow and desktop widths', async context => {
  const page = await open(context);
  for (const width of [320, 390, 739, 740, 1280]) {
    await page.setViewportSize({ width, height: 900 });
    await page.waitForFunction(() => {
      const svg = document.querySelector('#trace-map');
      return Math.abs(svg.viewBox.baseVal.width - svg.clientWidth) <= 1;
    });
    const overflow = await page.locator('#trace-map .station').evaluateAll(nodes =>
      nodes.flatMap(node => {
        const box = node.querySelector('rect').getBBox();
        return [...node.querySelectorAll('text')]
          .filter(label => label.getBBox().width > box.width - 6)
          .map(label => label.textContent);
      })
    );
    assert.deepEqual(overflow, [], `Station labels at ${width}px`);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth), width);
  }
});
