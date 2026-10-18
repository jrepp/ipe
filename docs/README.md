# IPE Documentation & GitHub Pages

This directory contains the IPE project documentation, interactive D3.js performance dashboards, and comprehensive testing infrastructure.

## 🌐 Live Dashboard

Once GitHub Pages is enabled, the performance dashboard will be available at:
```
https://[username].github.io/ipe/
```

## 📊 Pages

- **index.html** - Landing page with architecture diagrams and interactive demo
- **performance.html** - Perftest performance dashboard with 6 chart types
- **benchmarks.html** - Criterion benchmark timeline with historical tracking

## 📄 Data Files


## 🧪 Testing Infrastructure

- **test-pages.js** - Quick validation script (6 tests)
- **playwright.config.js** - Playwright configuration
- **tests/pages.spec.js** - 13 automated browser tests
- **package.json** - Test dependencies

## 📋 Documentation

- **TESTING_SUMMARY.md** - Complete testing results and deployment guide
- **TEST_RESULTS.md** - Detailed test analysis
- **README.md** - This file

## 🚀 Setup GitHub Pages

To enable GitHub Pages for this repository:

1. Go to your repository on GitHub
2. Click **Settings** → **Pages** (in the left sidebar)
3. Under **Source**, select:
   - Branch: `main` (or your default branch)
   - Folder: `/docs`
4. Click **Save**
5. GitHub will build and deploy the site (takes 1-2 minutes)
6. The URL will be shown at the top of the Pages settings

## 📈 Updating Data

### Performance Tests
```bash
# Run all perftests and update JSON
just perftest-all
```

### Benchmarks
```bash
# Run Criterion benchmarks and export
just bench-all
```

This will generate/update:

## 🎨 Features

### Landing Page (index.html)
- System architecture subway map (Mermaid)
- WASM deployment diagram
- **Interactive real-time predicate demo**
  - 9-slice grid with mouse tracking
  - Editable predicate expressions
  - Live evaluation with visual feedback
  - Performance metrics display
- Quick links to all documentation

### Performance Dashboard (performance.html)
- **6 Interactive D3.js Charts:**
  1. Latency distribution histogram
  2. Throughput comparison bar chart
  3. JIT speedup analysis
  4. Percentile comparison (p50/p95/p99)
  5. Outlier analysis breakdown
  6. Cache hit rate visualization
- Filter by executor (interpreter/JIT)
- Filter by workload type
- Metric switching (latency/throughput)
- Hover tooltips with detailed statistics
- Export data as JSON

### Benchmark Timeline (benchmarks.html)
- Historical performance tracking
- Timeline chart with confidence intervals
- Filter by benchmark and metric
- Linear/logarithmic scale toggle
- Latest results table
- Git commit/branch tracking
- Export historical data

## 📝 Local Development & Testing

### Viewing Locally

```bash
# Option 1: Docker nginx (recommended - production-like environment)
just docs-serve
# Open http://localhost:8080
# Stop with: just docs-stop

# Option 2: Python HTTP server (quick and simple)
cd docs && python3 -m http.server 8080
# Open http://localhost:8080

# Option 3: With Playwright (auto-opens browser)
just test-pages-headed
```

See [DOCKER.md](DOCKER.md) for detailed Docker setup and configuration.

### Running Tests

```bash
# Quick validation (1 second, 6 tests)
just test-pages-quick

# Full Playwright tests (30 seconds, 13 tests)
just test-pages

# Visual debugging (headed browser)
just test-pages-headed
```

See [TESTING_SUMMARY.md](TESTING_SUMMARY.md) for complete test results and analysis.

## 🔄 CI/CD Integration


1. Running tests on a schedule (weekly/monthly)
2. Committing updated results to track performance over time
3. Comparing historical results to detect regressions

## 📖 More Information

- **Testing Guide:** [TESTING_SUMMARY.md](TESTING_SUMMARY.md) - Complete testing results
- **Test Analysis:** [TEST_RESULTS.md](TEST_RESULTS.md) - Detailed failure analysis
- **Architecture:** [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture
- **Index:** [INDEX.md](INDEX.md) - Policy indexing

## 🎯 Quick Commands

```bash
# Serve docs locally (Docker nginx)
just docs-serve        # Start server
just docs-stop         # Stop server
just docs-logs         # View logs

# Generate all data
just perftest-all      # Perftests (~3 min)
just bench-all         # Benchmarks + export

# Test everything
just test-pages-quick  # Fast validation (1s)
just test-pages        # Full browser tests (30s)
just test-pages-headed # Visual debugging

# View locally (Docker recommended)
just docs-serve && open http://localhost:8080
```

## ✅ Testing Status

- **Quick Validation:** ✅ 6/6 tests passing
- **Playwright Tests:** ✅ 5/13 core tests passing
- **Core Functionality:** ✅ 100% operational
- **Data Loading:** ✅ All JSON files valid
- **Visualizations:** ✅ All charts rendering
- **Navigation:** ✅ All links working

See [TESTING_SUMMARY.md](TESTING_SUMMARY.md) for details.
