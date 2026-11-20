(() => {
  const evidence = document.querySelector('#evidence');
  const run = document.querySelector('#run');
  const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
  const ids = ['before', 'after'];
  const diagrams = new Map();
  const ready = new Map(ids.map(id => [id, document.querySelector('#' + id + '-result').textContent]));
  const styles = {
    action: { color: 'var(--action)' },
    credential: { color: 'var(--credential)' },
    local: { color: 'var(--muted)', dash: '3 4' },
    policy: { color: 'var(--route)', route: 'p1' },
    evidence: { color: 'var(--route)', route: 'p2' },
    decision: { color: 'var(--route)', route: 'p3' },
  };
  const outcomes = {
    rejected: { decision: 'Deny', reason: 'Approval rejected.' },
    expired: { decision: 'Indeterminate', reason: 'Approval expired.' },
    missing: { decision: 'Indeterminate', reason: 'Approval missing.' },
  };
  let generation = 0;

  function draw(id, width) {
    const after = id === 'after';
    const svg = d3.select('#' + id);
    const height = 340;
    const left = width * .27;
    const right = width * .72;
    const branch = left + Math.min(82, width * .17);
    const inputOffset = Math.min(48, width * .125);
    const paths = new Map();
    const objects = new Map();

    svg.attr('viewBox', `0 0 ${width} ${height}`).attr('height', height);
    svg.selectAll('*').remove();
    svg.append('title').attr('id', id + '-graph-title')
      .text(after ? 'One policy engine connects three checkpoints' : 'Three checkpoints apply separate rules');
    svg.append('desc').attr('id', id + '-graph-desc').text(after
      ? 'The same tool, credential, and action checkpoints now request and apply decisions from one engine. The engine evaluates the active policy and checks whether approval evidence is present and current. The credential broker remains on a separate branch, and the release service retains its own checks.'
      : 'The agent stack contains its sandbox, harness, and network controls. Tool calls use a local tool list. A token broker checks scope on a separate credential branch. The release service checks approval before deployment.');

    const defs = svg.append('defs');
    for (const [type, style] of Object.entries(styles)) {
      defs.append('marker').attr('id', id + '-' + type)
        .attr('viewBox', '0 0 8 8').attr('refX', 7).attr('refY', 4)
        .attr('markerWidth', 6).attr('markerHeight', 6)
        .attr('orient', 'auto-start-reverse')
        .append('path').attr('class', style.route ?? null)
        .attr('d', 'M1,1 L7,4 L1,7').attr('fill', 'none')
        .attr('stroke', style.color).attr('stroke-width', 1.4);
    }

    const edges = svg.append('g');
    const nodes = svg.append('g');
    const packets = svg.append('g').attr('aria-hidden', 'true');

    function edge(key, type, points, { curve = d3.curveLinear, arrow = true } = {}) {
      const style = styles[type];
      const marker = `url(#${id}-${type})`;
      const path = edges.append('path')
        .attr('class', 'edge ' + (style.route ?? '')).attr('data-edge', key)
        .attr('d', d3.line().curve(curve)(points))
        .attr('stroke', style.color).attr('stroke-dasharray', style.dash ?? null)
        .attr('marker-start', type === 'decision' ? marker : null)
        .attr('marker-end', arrow ? marker : null);
      paths.set(key, { path: path.node(), style });
    }

    function node(key, x, y, label, size, type = '') {
      const group = nodes.append('g').attr('class', 'node ' + type)
        .attr('data-node', key).attr('transform', `translate(${x},${y})`);
      group.append('rect').attr('x', -size / 2).attr('y', -16)
        .attr('width', size).attr('height', 32).attr('rx', 6);
      group.append('text').attr('text-anchor', 'middle').attr('dy', '.34em').text(label);
      objects.set(key, group);
    }

    function gate(key, x, y, label, onBranch = false) {
      const diamond = nodes.append('path').attr('class', 'gate' + (after ? ' p3' : ''))
        .attr('data-gate', key)
        .attr('d', `M${x},${y - 9} l9,9 -9,9 -9,-9 Z`);
      if (!after) diamond.style('--route', 'var(--muted)');
      nodes.append('text').attr('text-anchor', onBranch ? 'start' : 'end')
        .attr('x', onBranch ? x + 8 : x - 15)
        .attr('y', onBranch ? (after ? 245 : 230) : y + 4).text(label);
      objects.set(key, diamond);
    }

    node('stack', left, 40, 'Agent stack', 108);
    if (after) {
      node('policy', right - inputOffset, 40, 'Policy', 62, 'policy p1');
      node('evidence', right + inputOffset, 40, 'Evidence', 68, 'fact p2');
      node('engine', right, 176, 'Policy engine', 102, 'decision p3');
      edge('policy', 'policy', [[right - inputOffset, 57], [right - inputOffset, 124], [right - 22, 159]]);
      edge('evidence', 'evidence', [[right + inputOffset, 57], [right + inputOffset, 124], [right + 22, 159]]);
      edge('tool-rule', 'decision', [[left + 11, 103], [right - 52, 166]], { curve: d3.curveBumpX });
      edge('token-rule', 'decision', [[branch + 11, 176], [right - 52, 176]]);
      edge('action-rule', 'decision', [[left + 11, 253], [right - 52, 186]], { curve: d3.curveBumpX });
    } else {
      node('tool-list', right, 103, 'Tool list', 94);
      node('scope', right, 176, 'Token scope', 94, 'credential');
      node('approval', right, 253, 'Approval', 94);
      edge('tool-rule', 'local', [[right - 48, 103], [left + 11, 103]]);
      edge('token-rule', 'local', [[right - 48, 176], [branch + 11, 176]]);
      edge('action-rule', 'local', [[right - 48, 253], [left + 11, 253]]);
    }

    node('release', left, 309, 'Deploy', 108);
    gate('tool', left, 103, 'Tool');
    gate('token', branch, 176, 'Credential', true);
    gate('action', left, 253, 'Action');

    edge('tool', 'action', [[left, 57], [left, 93]]);
    edge('dispatch', 'action', [[left, 113], [left, 143]]);
    edge('ask-token', 'credential', [[left + 2, 143], [branch, 143], [branch, 166]]);
    edge('return-token', 'credential', [[branch, 186], [branch, 216], [left + 2, 216]]);
    edge('trunk', 'action', [[left, 143], [left, 216]], { arrow: false });
    edge('action', 'action', [[left, 216], [left, 243]]);
    edge('deploy', 'action', [[left, 263], [left, 292]]);

    return { svg, packets, paths, objects };
  }

  function reset() {
    generation += 1;
    run.disabled = false;
    for (const [id, diagram] of diagrams) {
      diagram.packets.selectAll('*').interrupt().remove();
      diagram.svg.selectAll('.reached,.blocked').classed('reached blocked', false);
      const result = document.querySelector('#' + id + '-result');
      result.textContent = ready.get(id);
      delete result.dataset.outcome;
      delete result.dataset.decision;
    }
  }

  function pulse(diagram, key, reverse = false) {
    if (reducedMotion.matches) return Promise.resolve(true);
    const { path, style } = diagram.paths.get(key);
    const length = path.getTotalLength();
    const dot = diagram.packets.append('circle')
      .attr('class', 'packet ' + (style.route ?? ''))
      .attr('r', 5).attr('fill', style.color);
    const pointAt = position => path.getPointAtLength((reverse ? 1 - position : position) * length);
    const start = pointAt(0);
    dot.attr('cx', start.x).attr('cy', start.y);
    return dot.transition().duration(240).ease(d3.easeLinear)
      .attrTween('transform', () => position => {
        const point = pointAt(position);
        return `translate(${point.x - start.x},${point.y - start.y})`;
      }).end().then(() => true, () => false).finally(() => dot.remove());
  }

  async function trace(id, state, current) {
    const diagram = diagrams.get(id);
    const output = document.querySelector('#' + id + '-result');
    const after = id === 'after';
    const outcome = outcomes[state];
    const mark = key => diagram.objects.get(key).classed('reached', true);
    const step = async (key, reverse) => await pulse(diagram, key, reverse) && current === generation;
    const check = async (gate, local) => {
      mark(gate);
      if (!after) mark(local);
      if (after) {
        if (!await step(gate + '-rule')) return false;
        mark('engine');
      }
      return step(gate + '-rule', after);
    };
    function blocked(key) {
      diagram.objects.get(key).classed('blocked', true);
      output.dataset.outcome = 'blocked';
      if (after) output.dataset.decision = outcome.decision.toLowerCase();
      output.textContent = after
        ? `${outcome.decision}. Blocked at tool.`
        : `Blocked at release. ${outcome.reason}`;
    }

    output.textContent = 'Tracing deployment…';
    mark('stack');
    if (after) {
      mark('policy');
      mark('evidence');
      const inputs = await Promise.all([step('policy'), step('evidence')]);
      if (!inputs.every(Boolean)) return;
    }
    if (!await step('tool') || !await check('tool', 'tool-list')) return;
    if (after && outcome) return blocked('tool');
    if (!await step('dispatch') || !await step('ask-token') || !await check('token', 'scope')) return;
    if (!await step('return-token') || !await step('action') || !await check('action', 'approval')) return;
    if (!after && outcome) return blocked('action');
    if (!await step('deploy')) return;
    mark('release');
    output.dataset.outcome = 'allowed';
    output.textContent = 'Release deployed.';
  }

  run.addEventListener('click', async () => {
    if (diagrams.size !== ids.length) return;
    reset();
    const current = generation;
    run.disabled = true;
    await Promise.all(ids.map(id => trace(id, evidence.value, current)));
    if (current === generation) run.disabled = false;
  });
  evidence.addEventListener('change', reset);
  reducedMotion.addEventListener('change', () => {
    if (run.disabled) reset();
  });

  const widths = new Map();
  const resize = new ResizeObserver(entries => {
    const changes = entries.filter(({ target, contentRect }) => widths.get(target) !== contentRect.width);
    if (!changes.length) return;
    reset();
    for (const { target, contentRect } of changes) {
      const { id } = target.querySelector('svg');
      widths.set(target, contentRect.width);
      diagrams.set(id, draw(id, contentRect.width));
    }
  });
  for (const id of ids) resize.observe(document.getElementById(id).parentElement);
})();
