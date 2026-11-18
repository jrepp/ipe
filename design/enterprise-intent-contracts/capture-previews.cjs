const { chromium } = require('playwright');
const { mkdir } = require('node:fs/promises');
const { join } = require('node:path');
const { pathToFileURL } = require('node:url');

(async () => {
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, reducedMotion: 'reduce' });
    await page.goto(pathToFileURL(join(__dirname, 'enterprise-intent-contracts.html')).href);
    await page.evaluate(() => document.fonts.ready);
    await page.locator('#modeling-after [data-node="handoff"]').waitFor();
    await page.locator('#after [data-node="engine"]').waitFor();
    const destination = join(__dirname, 'previews');
    await mkdir(destination, { recursive: true });
    await page.screenshot({ path: join(destination, '00-projects.png'), fullPage: true });
    await page.locator('.modeling').screenshot({ path: join(destination, '01-modeling.png') });
    await page.locator('.enforcement').screenshot({ path: join(destination, '02-enforcement.png') });
    await page.goto(pathToFileURL(join(__dirname, 'policy-enforcement.html')).href);
    await page.evaluate(() => document.fonts.ready);
    await page.locator('#trace-map [data-station="engine"]').waitFor();
    await page.screenshot({ path: join(destination, '03-enforcement-detail.png'), fullPage: true });
    console.log('Saved the project overview, comparisons, and enforcement detail in their initial sample states.');
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
