/**
 * Test to trigger ClusterService calls
 */

const { test } = require('@playwright/test');

test('navigate to fabrics/clusters to trigger ClusterService error', async ({ page }) => {
  // Listen for console errors
  page.on('console', msg => {
    if (msg.type() === 'error') {
      console.log('❌ Console Error:', msg.text());
    }
  });

  // Listen for failed requests
  page.on('requestfailed', request => {
    console.log('❌ Network Error:', request.url(), request.failure());
  });

  // Listen for responses
  page.on('response', async response => {
    // Log API responses, especially error ones
    if (response.url().includes('/api/') || response.url().includes('grpc')) {
      console.log(`📡 API Response: ${response.status()} ${response.url()}`);

      // Try to get response body for errors
      if (response.status() >= 400) {
        try {
          const body = await response.text();
          console.log('Response body:', body.substring(0, 500));
        } catch (e) {
          // ignore
        }
      }
    }
  });

  // Go to admin console
  await page.goto('http://localhost:8090');
  console.log('✓ Loaded dashboard');

  // Wait for page to load
  await page.waitForTimeout(1000);

  // Try clicking on Fabrics link
  const fabricsLink = page.locator('a:has-text("Fabrics")').first();
  const fabricsExists = await fabricsLink.isVisible().catch(() => false);

  if (fabricsExists) {
    console.log('✓ Found Fabrics link, clicking...');
    await fabricsLink.click();

    // Wait for navigation and content to load
    await page.waitForTimeout(2000);

    // Take screenshot
    await page.screenshot({ path: 'docs/test-results/fabrics-page.png', fullPage: true });

    // Get page content
    const bodyText = await page.locator('body').textContent();
    console.log('\n=== Fabrics Page Content (first 1500 chars) ===');
    console.log(bodyText.substring(0, 1500));

    // Look for the specific error
    const errorVisible = await page.locator('text=/Service Not Implemented|Unimplemented|ClusterService/i')
      .isVisible()
      .catch(() => false);

    if (errorVisible) {
      console.log('\n❌ Found "Service Not Implemented" error!');

      // Get the full error message
      const errorText = await page.locator('text=/Service Not Implemented|Unimplemented/i')
        .first()
        .textContent()
        .catch(() => null);

      if (errorText) {
        console.log('Error message:', errorText);
      }

      // Look for technical details
      const detailsButton = page.locator('button:has-text("Technical Details"), summary:has-text("Technical Details")').first();
      const detailsExists = await detailsButton.isVisible().catch(() => false);

      if (detailsExists) {
        console.log('✓ Found technical details section, expanding...');
        await detailsButton.click();
        await page.waitForTimeout(500);

        const details = await page.locator('text=/rpc error.*ClusterService/i')
          .textContent()
          .catch(() => 'Could not find ClusterService error text');

        console.log('\n=== Technical Details ===');
        console.log(details);
      }

      // Take another screenshot showing the error
      await page.screenshot({ path: 'docs/test-results/cluster-service-error.png', fullPage: true });
      console.log('\n✓ Screenshot saved to docs/test-results/cluster-service-error.png');
    } else {
      console.log('\n✓ No error found on Fabrics page');
    }
  } else {
    console.log('❌ Fabrics link not found');
  }

  // Try clicking on any cluster-related elements
  const clusterLinks = await page.locator('a:has-text("Cluster"), button:has-text("Cluster")').all();

  if (clusterLinks.length > 0) {
    console.log(`\n✓ Found ${clusterLinks.length} cluster-related elements`);

    for (let i = 0; i < Math.min(clusterLinks.length, 3); i++) {
      const link = clusterLinks[i];
      const text = await link.textContent();
      console.log(`  - Clicking: "${text}"`);

      await link.click();
      await page.waitForTimeout(1000);

      // Check for error
      const errorAfterClick = await page.locator('text=/Service Not Implemented|Unimplemented/i')
        .isVisible()
        .catch(() => false);

      if (errorAfterClick) {
        console.log('    ❌ Error appeared after clicking this element!');
        await page.screenshot({ path: `docs/test-results/cluster-error-${i}.png`, fullPage: true });
        break;
      }
    }
  }
});
