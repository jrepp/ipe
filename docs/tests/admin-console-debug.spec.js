/**
 * Debug test to capture the admin console state
 */

const { test } = require('@playwright/test');

test('capture admin console state', async ({ page }) => {
  await page.goto('http://localhost:8090');

  // Wait for page to load
  await page.waitForTimeout(2000);

  // Take screenshot
  await page.screenshot({ path: 'docs/test-results/admin-console-screenshot.png', fullPage: true });

  // Get page HTML
  const html = await page.content();
  console.log('=== Admin Console HTML (first 2000 chars) ===');
  console.log(html.substring(0, 2000));

  // Get page title
  const title = await page.title();
  console.log('\n=== Page Title ===');
  console.log(title);

  // Check for any visible text
  const bodyText = await page.locator('body').textContent();
  console.log('\n=== Visible Text (first 1000 chars) ===');
  console.log(bodyText.substring(0, 1000));

  // Look for error messages
  const errorElements = await page.locator('[class*="error"], [class*="warning"], [role="alert"]').all();
  if (errorElements.length > 0) {
    console.log('\n=== Error/Warning Elements ===');
    for (const el of errorElements) {
      const text = await el.textContent();
      console.log(text);
    }
  }

  // Check network requests
  const responses = [];
  page.on('response', response => {
    responses.push({
      url: response.url(),
      status: response.status(),
      statusText: response.statusText()
    });
  });

  // Trigger any API calls by waiting a bit more
  await page.waitForTimeout(1000);

  if (responses.length > 0) {
    console.log('\n=== Network Responses ===');
    responses.forEach(r => {
      console.log(`${r.status} ${r.statusText} - ${r.url}`);
    });
  }
});
