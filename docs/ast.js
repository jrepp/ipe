(() => {
  const root = document.getElementById('ast-tree');
  if (!root) return;
  const descriptions = {
    Policy: 'The complete policy: its intent, trigger conditions, and requirements.',
    Intent: 'A human-readable explanation of why this policy exists.',
    Triggers: 'Conditions that determine when this policy applies.',
    Condition: 'A wrapper around an expression in a trigger or requirement.',
    Requires: 'Conditions that must hold for access to be allowed.',
    Denies: 'A denial clause with an optional reason.',
    Where: 'Additional conditions on the requirement.',
    Binary: 'A comparison between the left and right expression.',
    Logical: 'Combines or negates boolean expressions.',
    Path: 'A field reference resolved against the evaluation context.',
    Literal: 'A typed constant written directly in the source.',
    In: 'Tests whether the expression belongs to a set of values.',
    Array: 'The candidate values in a membership test.',
  };
  let examples;
  let selected;
  function select(node, button) {
    if (selected) selected.removeAttribute('data-selected');
    selected = button;
    button.dataset.selected = 'true';
    const detail = document.getElementById('ast-detail');
    if (detail) {
      detail.textContent = JSON.stringify(node, null, 2);
      document.getElementById('ast-explanation').textContent = descriptions[node.kind] || 'An expression node in the policy.';
    }
  }
  function render(index) {
    const example = examples[index];
    let count = 0;
    function branch(node) {
      count++;
      const li = document.createElement('li');
      const button = document.createElement('button');
      button.type = 'button'; button.className = 'ast-node'; button.dataset.kind = node.kind;
      const kind = document.createElement('b'); kind.textContent = node.kind;
      const label = document.createElement('span'); label.textContent = node.label;
      button.append(kind, label);
      button.addEventListener('click', event => { event.preventDefault(); select(node, button); });
      if (node.children.length) {
        const details = document.createElement('details'); details.open = true;
        const summary = document.createElement('summary'); summary.append(button);
        const list = document.createElement('ul'); node.children.forEach(child => list.append(branch(child)));
        details.append(summary, list); li.append(details);
      } else li.append(button);
      return li;
    }
    const list = document.createElement('ul'); list.className = 'ast-tree'; list.append(branch(example.tree));
    root.replaceChildren(list);
    const source = document.getElementById('ast-source'); if (source) source.textContent = example.source;
    const counter = document.getElementById('ast-count'); if (counter) counter.textContent = `${count} nodes`;
    select(example.tree, root.querySelector('button'));
  }
  fetch('data/site-data.json').then(response => { if (!response.ok) throw new Error('Parser output is unavailable.'); return response.json(); }).then(data => {
    examples = data.examples;
    const selector = document.getElementById('ast-example');
    if (selector) {
      selector.replaceChildren(...examples.map((example, index) => new Option(example.name, index)));
      selector.disabled = false; selector.addEventListener('change', () => render(Number(selector.value)));
    }
    render(0);
    const status = document.getElementById('ast-status');
    if (status) status.textContent = `Parser output · ${data.revision}${data.working_tree_modified ? ' + local changes' : ''} · ${new Date(data.generated_at).toLocaleString()}`;
    for (const [id, open] of [['ast-expand', true], ['ast-collapse', false]]) {
      document.getElementById(id)?.addEventListener('click', () => root.querySelectorAll('details').forEach(item => { item.open = open; }));
    }
  }).catch(error => { root.textContent = `${error.message} Run make docs-site, then serve the repository with make docs-serve.`; root.classList.add('error'); });
})();
