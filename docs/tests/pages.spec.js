const { test, expect } = require('@playwright/test');

test.describe('IPE Documentation Pages', () => {

  test('index page loads and displays correctly', async ({ page }) => {
    await page.goto('/');

    // Check title
    await expect(page).toHaveTitle(/IPE - Intent Policy Engine/);

    // Check main heading
    await expect(page.locator('h1')).toHaveText('IPE');

    // Check navigation cards exist (use more specific selectors to avoid multiple matches)
    await expect(page.locator('.card-title:has-text("Performance Dashboard")')).toBeVisible();
    await expect(page.locator('.card-title:has-text("Benchmark Timeline")')).toBeVisible();
    await expect(page.locator('.card-title:has-text("Architecture")')).toBeVisible();

    // Check Mermaid diagrams load
    const diagrams = page.locator('.mermaid');
    await expect(diagrams).toHaveCount(2); // System architecture + WASM deployment

    // Check interactive demo elements exist
    await expect(page.locator('#demo-grid')).toBeVisible();
    await expect(page.locator('#predicate-editors')).toBeVisible();
    await expect(page.locator('#reset-btn')).toBeVisible();

    // Check metrics display
    await expect(page.locator('#mouse-pos')).toBeVisible();
    await expect(page.locator('#eval-rate')).toBeVisible();
  });

  test('interactive demo responds to mouse movement', async ({ page }) => {
    await page.goto('/');

    const grid = page.locator('#demo-grid');
    await expect(grid).toBeVisible();

    // Move mouse over the grid
    const gridBox = await grid.boundingBox();
    if (gridBox) {
      // Move to center of grid
      await page.mouse.move(gridBox.x + gridBox.width / 2, gridBox.y + gridBox.height / 2);
      await page.waitForTimeout(500); // Give time for event handlers

      // Wait for mouse position to update (not be '-')
      await page.waitForFunction(() => {
        const elem = document.getElementById('mouse-pos');
        return elem && elem.textContent !== '-';
      }, { timeout: 2000 });

      // Check that at least one cell is active
      const activeAfter = await page.locator('#active-cells').textContent();
      expect(activeAfter).toMatch(/[1-9]\/9/); // At least 1 active cell
    }
  });

  test('interactive demo reset button works', async ({ page }) => {
    await page.goto('/');

    // Click reset button
    await page.click('#reset-btn');

    // Verify predicates are reset (should see default predicates)
    const firstEditor = page.locator('#predicate-editors textarea').first();
    const value = await firstEditor.inputValue();
    expect(value).toContain('mouse.x < 200');
  });

  test('performance dashboard loads and displays data', async ({ page }) => {
    await page.goto('/performance.html');

    // Check title (full title includes "Predicate Execution")
    await expect(page).toHaveTitle(/IPE Predicate Execution Performance Dashboard/);

    // Wait for data to load
    await page.waitForSelector('#content', { state: 'visible', timeout: 5000 });

    // Check that loading message is gone
    await expect(page.locator('#loading')).not.toBeVisible();

    // Check summary stats are displayed
    await expect(page.locator('.stat-card')).toHaveCount(5); // Updated to 5

    // Check that charts container exists
    await expect(page.locator('.chart-container')).toHaveCount(7); // 7 charts now

    // Verify at least one chart rendered
    const svgElements = page.locator('svg');
    await expect(svgElements.first()).toBeVisible();
  });

  // Note: Performance dashboard no longer has executor/workload filters
  // Charts display all data without filtering

  test('performance dashboard tooltips work', async ({ page }) => {
    await page.goto('/performance.html');

    await page.waitForSelector('#content', { state: 'visible' });

    // Find a data point and hover over it
    const firstDot = page.locator('.dot').first();
    if (await firstDot.count() > 0) {
      await firstDot.hover();

      // Check tooltip appears
      const tooltip = page.locator('#tooltip');
      await expect(tooltip).toBeVisible();
    }
  });

  test('benchmarks page loads and displays data', async ({ page }) => {
    await page.goto('/benchmarks.html');

    // Check title
    await expect(page).toHaveTitle(/IPE Benchmark Timeline/);

    // Wait for data to load
    await page.waitForSelector('#content', { state: 'visible', timeout: 5000 });

    // Check that loading is gone
    await expect(page.locator('#loading')).not.toBeVisible();

    // Check summary stats
    await expect(page.locator('.stat-card')).toHaveCount(5);

    // Check controls exist
    await expect(page.locator('#benchmark-select')).toBeVisible();
    await expect(page.locator('#metric-select')).toBeVisible();
    await expect(page.locator('#scale-select')).toBeVisible();

    // Check timeline chart exists
    await expect(page.locator('#timeline-chart')).toBeVisible();

    // Check latest results table exists
    await expect(page.locator('#latest-results table')).toBeVisible();
  });

  test('benchmarks page filters work', async ({ page }) => {
    await page.goto('/benchmarks.html');

    await page.waitForSelector('#content', { state: 'visible' });

    // Get available options from benchmark select
    const options = await page.locator('#benchmark-select option').count();
    expect(options).toBeGreaterThan(0);

    // Change metric
    await page.selectOption('#metric-select', 'median_ns');

    // Change scale
    await page.selectOption('#scale-select', 'log');

    // Chart auto-updates on select change (no "Update Chart" button)
    // Wait for chart to be visible after filter changes
    await expect(page.locator('#timeline-chart svg')).toBeVisible();
  });

  test('navigation between pages works', async ({ page }) => {
    // Start at index
    await page.goto('/');

    // Click Performance Dashboard link
    await page.click('a[href="performance.html"]');
    await expect(page).toHaveURL(/performance.html/);
    await expect(page.locator('h1')).toContainText('Performance Dashboard');

    // Go back to home
    await page.click('a:has-text("Back to Home")');
    await expect(page).toHaveURL(/\/$|\/index.html$/);

    // Click Benchmark Timeline link
    await page.click('a[href="benchmarks.html"]');
    await expect(page).toHaveURL(/benchmarks.html/);
    await expect(page.locator('h1')).toContainText('Benchmark Timeline');

    // Go back to home
    await page.click('a:has-text("Back to Home")');
    await expect(page).toHaveURL(/\/$|\/index.html$/);
  });

  test('all external resources load correctly', async ({ page }) => {
    await page.goto('/');

    // Check D3.js loads
    const d3Loaded = await page.evaluate(() => {
      return typeof window.d3 !== 'undefined';
    });
    expect(d3Loaded).toBe(true);

    // Check Mermaid loads
    const mermaidLoaded = await page.evaluate(() => {
      return typeof window.mermaid !== 'undefined';
    });
    expect(mermaidLoaded).toBe(true);
  });

  test('performance dashboard export functionality', async ({ page }) => {
    await page.goto('/performance.html');
    await page.waitForSelector('#content', { state: 'visible' });

    // Set up download listener
    const downloadPromise = page.waitForEvent('download');

    // Click export button
    await page.click('button:has-text("Export Data")');

    // Wait for download
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toMatch(/perftest-export.*\.json/);
  });

  test('benchmarks page export functionality', async ({ page }) => {
    await page.goto('/benchmarks.html');
    await page.waitForSelector('#content', { state: 'visible' });

    // Set up download listener
    const downloadPromise = page.waitForEvent('download');

    // Click export button
    await page.click('button:has-text("Export Data")');

    // Wait for download
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toMatch(/benchmark-history.*\.json/);
  });

  test('responsive design works on mobile', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });

    await page.goto('/');

    // Check that content is still visible
    await expect(page.locator('h1')).toBeVisible();
    await expect(page.locator('.cards')).toBeVisible();

    // Check grid layout adapts
    const grid = page.locator('.cards').first();
    const gridStyles = await grid.evaluate(el => {
      const styles = window.getComputedStyle(el);
      return styles.display;
    });
    expect(gridStyles).toBe('grid');
  });
});
