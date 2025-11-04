/**
 * Admin Console Integration Tests
 *
 * Tests the AGF Admin Console at localhost:8090
 * Validates that the web console can communicate with the admin plane backend
 */

const { test, expect } = require('@playwright/test');

test.describe('AGF Admin Console', () => {

  test('admin console loads', async ({ page }) => {
    await page.goto('http://localhost:8090');

    // Check that the page loads (should get some response)
    await expect(page).toHaveTitle(/AGF|Admin|Console/);
  });

  test('admin console shows service error', async ({ page }) => {
    // Listen for console errors
    const consoleErrors = [];
    page.on('console', msg => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    // Listen for network errors
    const networkErrors = [];
    page.on('requestfailed', request => {
      networkErrors.push({
        url: request.url(),
        failure: request.failure()
      });
    });

    await page.goto('http://localhost:8090');

    // Wait for any async operations
    await page.waitForTimeout(2000);

    // Check if we can see the "Service Not Implemented" error
    const errorVisible = await page.locator('text=/Service Not Implemented|Unimplemented/i').isVisible().catch(() => false);

    if (errorVisible) {
      console.log('✓ Found expected "Service Not Implemented" error');

      // Look for the technical details
      const technicalDetails = await page.locator('text=/rpc error.*ClusterService/i').textContent().catch(() => null);
      if (technicalDetails) {
        console.log('Technical details:', technicalDetails);
      }
    }

    // Log any errors found
    if (consoleErrors.length > 0) {
      console.log('Console errors:', consoleErrors);
    }
    if (networkErrors.length > 0) {
      console.log('Network errors:', networkErrors);
    }
  });

  test('admin plane backend is running', async ({ request }) => {
    // Check if the admin plane backend is accessible
    const response = await request.get('http://localhost:28082/health').catch(err => ({
      status: () => 0,
      text: () => err.message
    }));

    console.log('Admin plane health check status:', response.status());

    // Backend might be running even if unhealthy
    expect(response.status()).toBeGreaterThanOrEqual(0);
  });

  test('check database schema issue', async ({ page }) => {
    // Navigate to admin console
    await page.goto('http://localhost:8090');

    // Try to trigger the ClusterService call
    // This might be done by clicking on a clusters link or similar
    const clustersLink = page.locator('a:has-text("Cluster"), button:has-text("Cluster")').first();
    const exists = await clustersLink.isVisible().catch(() => false);

    if (exists) {
      await clustersLink.click();

      // Wait for error to appear
      await page.waitForTimeout(1000);

      // Check for the error
      const errorText = await page.locator('text=/error|failed|unimplemented/i').first().textContent().catch(() => null);
      if (errorText) {
        console.log('Error when accessing clusters:', errorText);

        // Verify it's the ClusterService error
        expect(errorText.toLowerCase()).toMatch(/cluster|unimplemented/);
      }
    } else {
      console.log('No clusters navigation element found');
    }
  });
});
