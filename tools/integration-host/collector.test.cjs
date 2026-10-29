const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const root = path.resolve(__dirname, '../..');
const read = file => fs.readFileSync(path.join(root, file), 'utf8');
const sdk = read('crates/ipe-integration/sdk/index.js');
function evaluate(program, expression, input) {
  const context = vm.createContext({input: JSON.stringify(input)});
  return JSON.parse(vm.runInContext(`${sdk}\n${program}\nJSON.stringify(${expression})`, context, {timeout: 1000}));
}
const hosted = read('testing/integration/hosted/github-actions.js');
const standalone = read('crates/ipe-integration/adapters/github-contract/project.js');
const hostedCall = 'projectDeclaration(JSON.parse(input).kind, JSON.parse(input).document)';
const standaloneCall = "step({}, [{id:'source', receipt:'read', data:[JSON.parse(input)]}], {}).outputs";

test('hosted projection preserves standalone declarations, references, and gaps', () => {
  for (const count of [0, 1, 6, 20]) for (const kind of ['workflow', 'action']) {
    const steps = Array.from({length: count}, (_, i) => ({
      uses: 'fixture/action@v1', with: {artifact: `item${i}`}, if: '${{ success() }}',
      unrecognized: {nested: ['x', true, null]},
    }));
    const document = kind === 'workflow'
      ? {name: 'Sample', on: 'push', permissions: {contents: 'read'}, unknown: {nested: 1},
        jobs: {'a~/b': {steps, strategy: {matrix: {os: ['linux', 'macos']}}, needs: ['other'], environment: 'pages', unrecognized: 'retained'}}}
      : {name: 'Action', inputs: {name: {required: true, default: 'value'}}, outputs: {value: {}},
        runs: {using: count % 2 ? 'composite' : 'node24', main: 'index.js', steps, unknown: {nested: 1}}};
    const input = {kind, document};
    const expected = evaluate(standalone, standaloneCall, input);
    if (kind === 'workflow') {
      // The host also captures run results; the standalone declaration reader does not.
      assert.deepEqual(expected.gaps, [{pointer: '', code: 'repository_rules_environment_protection_and_run_results_not_captured'}]);
      expected.gaps = [{pointer: '', code: 'repository_rules_and_environment_protection_not_captured'}];
    }
    assert.deepEqual(evaluate(hosted, hostedCall, input), expected);
  }
});

test('hosted projection still rejects malformed job and action objects', () => {
  for (const input of [
    {kind: 'workflow', document: {jobs: {bad: 'string'}}},
    {kind: 'workflow', document: {jobs: {bad: []}}},
    {kind: 'action', document: {runs: []}},
  ]) {
    assert.throws(() => evaluate(hosted, hostedCall, input));
    assert.throws(() => evaluate(standalone, standaloneCall, input));
  }
});
