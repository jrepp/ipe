#!/usr/bin/env node
/**
 * Simple validation script to test that all GitHub Pages components are working
 */

const fs = require('fs');
const path = require('path');

console.log('🧪 Testing GitHub Pages Setup\n');

const tests = [
  {
    name: 'perftest-results.json',
    check: (data) => {
      const json = JSON.parse(data);
      return json.total_tests > 0 && json.results && json.results.length > 0;
    }
  },
  {
    name: 'benchmark-latest.json',
    check: (data) => {
      const json = JSON.parse(data);
      return json.benchmarks && json.benchmarks.length > 0;
    }
  },
  {
    name: 'benchmark-history.json',
    check: (data) => {
      const json = JSON.parse(data);
      return Array.isArray(json) && json.length > 0;
    }
  },
  {
    name: 'index.html',
    check: (data) => {
      return data.includes('IPE - Intent Policy Engine') &&
             data.includes('performance.html') &&
             data.includes('benchmarks.html') &&
             data.includes('mermaid');
    }
  },
  {
    name: 'performance.html',
    check: (data) => {
      return data.includes("fetch('perftest-results.json')") &&
             data.includes('d3js.org') &&
             data.includes('Performance Dashboard');
    }
  },
  {
    name: 'benchmarks.html',
    check: (data) => {
      return data.includes("fetch('benchmark-history.json')") &&
             data.includes("fetch('benchmark-latest.json')") &&
             data.includes('d3js.org') &&
             data.includes('Benchmark Timeline');
    }
  }
];

let passed = 0;
let failed = 0;

tests.forEach(test => {
  try {
    const filePath = path.join(__dirname, test.name);
    const data = fs.readFileSync(filePath, 'utf8');

    if (test.check(data)) {
      console.log(`✅ ${test.name}`);
      passed++;
    } else {
      console.log(`❌ ${test.name} - validation failed`);
      failed++;
    }
  } catch (error) {
    console.log(`❌ ${test.name} - ${error.message}`);
    failed++;
  }
});

console.log(`\n📊 Results: ${passed} passed, ${failed} failed`);

if (failed === 0) {
  console.log('\n🎉 All tests passed! GitHub Pages setup is ready.');
  console.log('\nNext steps:');
  console.log('1. Commit the docs/ directory');
  console.log('2. Push to GitHub');
  console.log('3. Enable GitHub Pages in repository settings (source: /docs)');
  console.log('4. Visit https://yourusername.github.io/ipe/');
} else {
  console.log('\n⚠️  Some tests failed. Please review the errors above.');
  process.exit(1);
}
