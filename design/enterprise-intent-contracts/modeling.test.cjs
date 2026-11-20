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
  await page.locator('#modeling-after [data-node="handoff"]').waitFor();
  return page;
}

test('planning stops at the source rejection or the pending human handoff', async context => {
  const page = await open(context, { reducedMotion: 'no-preference' });
  const run = page.locator('#modeling-run');
  const packets = page.locator('.modeling .packet');

  for (const reducedMotion of ['no-preference', 'reduce']) {
    await page.emulateMedia({ reducedMotion });
    await run.press('Enter');
    if (reducedMotion === 'no-preference') await packets.first().waitFor();
    await page.locator('#modeling-before-result[data-outcome="blocked"]').waitFor();
    await page.locator('#modeling-after-result[data-outcome="pending"]').waitFor();
    await page.waitForFunction(() => !document.querySelector('#modeling-run').disabled);
    assert.equal(await packets.count(), 0);
    assert.equal(await page.locator('#modeling-before [data-node="error"].visited').count(), 1);
    assert.equal(await page.locator('#modeling-after [data-node="handoff"].visited').count(), 1);
    assert.equal(await page.locator('#modeling-after [data-node="service"]').count(), 0);
    assert.equal(await page.locator('#after .node.reached').count(), 0);
  }

  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await run.press('Enter');
  await packets.first().waitFor();
  await page.setViewportSize({ width: 390, height: 920 });
  await page.waitForFunction(() => !document.querySelector('#modeling-run').disabled);
  assert.equal(await packets.count(), 0);
});

test('modeling labels fit at narrow and desktop widths in both color schemes', async context => {
  const page = await open(context);
  for (const colorScheme of ['light', 'dark']) {
    await page.emulateMedia({ colorScheme });
    for (const width of [320, 390, 840, 841, 1280]) {
      await page.setViewportSize({ width, height: 920 });
      await page.waitForFunction(() => [...document.querySelectorAll('.modeling svg')]
        .every(svg => Math.abs(svg.viewBox.baseVal.width - svg.clientWidth) <= 1));
      const textIssues = await page.locator('.modeling svg text').evaluateAll(labels =>
        labels.flatMap(label => {
          const svg = label.ownerSVGElement;
          const bounds = label.getBoundingClientRect();
          const canvas = svg.getBoundingClientRect();
          const scale = svg.clientWidth / svg.viewBox.baseVal.width;
          const fontSize = parseFloat(getComputedStyle(label).fontSize) * scale;
          return bounds.left < canvas.left || bounds.right > canvas.right || fontSize < 11
            ? [label.textContent]
            : [];
        })
      );
      assert.deepEqual(textIssues, [], `${colorScheme}, ${width}px`);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth), width);
    }
  }
});
