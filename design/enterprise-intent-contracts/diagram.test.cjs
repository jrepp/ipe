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
  await page.goto(new URL('./index.html', pathToFileURL(__filename)).href);
  await page.locator('#after [data-node="engine"]').waitFor();
  return page;
}

test('enforcement allows one case and stops the three blocked cases before deployment', async context => {
  const page = await open(context, { reducedMotion: 'reduce' });
  const cases = [
    ['approved', 'allowed', null, 3, 1],
    ['rejected', 'blocked', 'deny', 1, 0],
    ['expired', 'blocked', 'indeterminate', 1, 0],
    ['missing', 'blocked', 'indeterminate', 1, 0],
  ];
  for (const [state, outcome, decision, gates, release] of cases) {
    await page.locator('#evidence').selectOption(state);
    await page.locator('#run').press('Enter');
    await page.locator(`#after-result[data-outcome="${outcome}"]`).waitFor();
    assert.equal(await page.locator('#after-result').getAttribute('data-decision'), decision);
    assert.equal(await page.locator('#before-result').getAttribute('data-outcome'), outcome);
    assert.equal(await page.locator('#after .gate.reached').count(), gates);
    assert.equal(await page.locator('#after [data-node="release"].reached').count(), release);
    assert.equal(await page.locator('#after [data-gate="token"].reached').count(), release);
    assert.equal(await page.locator('#before [data-gate="action"].reached').count(), 1);
    assert.equal(await page.locator('#before [data-node="release"].reached').count(), release);
    assert.equal(await page.locator('svg .packet').count(), 0);
  }
});

test('an animated trace can be cancelled and replayed', async context => {
  const page = await open(context, { reducedMotion: 'no-preference' });
  await page.locator('#run').click();
  await page.locator('#after .packet').first().waitFor();
  await page.locator('#evidence').selectOption('expired');
  assert.equal(await page.locator('#after-result').getAttribute('data-outcome'), null);
  assert.equal(await page.locator('svg .packet').count(), 0);
  await page.locator('#run').click();
  await page.locator('#after-result[data-outcome="blocked"]').waitFor();
  await page.locator('#before-result[data-outcome="blocked"]').waitFor();
  assert.equal(await page.locator('svg .packet').count(), 0);
  assert.equal(await page.locator('#run').isEnabled(), true);
});

test('enforcement labels fit their nodes and retain the site palette', async context => {
  const page = await open(context, { colorScheme: 'dark' });
  for (const width of [320, 390, 841, 1280]) {
    await page.setViewportSize({ width, height: 920 });
    await page.waitForFunction(() => {
      const svg = document.querySelector('#after');
      return Math.abs(svg.viewBox.baseVal.width - svg.clientWidth) <= 1;
    });
    const overflow = await page.locator('#before .node, #after .node').evaluateAll(nodes =>
      nodes.flatMap(node => {
        const box = node.querySelector('rect').getBBox();
        return [...node.querySelectorAll('text')]
          .filter(label => label.getBBox().width > box.width - 6)
          .map(label => label.textContent);
      })
    );
    assert.deepEqual(overflow, [], `Overlapping node text at ${width}px`);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth), width);
  }
  assert.equal(await page.locator('#after .node rect').first().evaluate(node => getComputedStyle(node).fill), 'rgb(255, 255, 255)');
});
