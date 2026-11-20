(() => {
  const scenario = document.querySelector('#trace-scenario');
  const run = document.querySelector('#trace-run');
  const result = document.querySelector('#trace-result');
  const canvas = document.querySelector('.detail-canvas');
  const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
  const idle = result.textContent;
  const palette = {
    transport: { color: 'var(--muted)' },
    policy: { color: 'var(--route)', route: 'p1' },
    evidence: { color: 'var(--route)', route: 'p2' },
    publish: { color: 'var(--route)', route: 'p2', dash: '5 4' },
    decision: { color: 'var(--route)', route: 'p3' },
  };
  const outcomes = {
    allowed: ['allowed', 'Allow', 'Service applied allow. The sample release proceeded.'],
    denied: ['blocked', 'Deny', 'Policy denied. The service blocked the release.'],
    stale: ['blocked', 'Indeterminate', 'Evidence expired. The service blocked the release.'],
    jwt: ['blocked', 'JWT rejected', 'JWT validation failed. The service sent no evaluation request.'],
    endpoint: ['blocked', 'Route rejected', 'The endpoint did not match a trusted route. The service sent no evaluation request.'],
    unavailable: ['blocked', 'No decision', 'Engine unavailable. The service blocked the release.'],
  };
  let diagram;
  let width;
  let generation = 0;

  function arrange(w) {
    if (w >= 740) {
      const token = w * .14;
      const service = w * .44;
      const engine = w * .75;
      return {
        height: 354,
        gate: [service, 148],
        positions: {
          jwt: [token, 72, 152], service: [service, 72, 188], engine: [engine, 72, 230],
          registry: [token, 223, 152], action: [service, 223, 154],
          policy: [engine - 83, 223, 150], facts: [engine + 83, 223, 150],
          publisher: [engine + 83, 316, 150],
        },
        links: {
          jwt: [[token + 77, 72], [service - 95, 72]],
          registry: [[token + 77, 223], [service - 111, 223], [service - 111, 91], [service - 95, 91]],
          rar: [[service + 95, 57], [engine - 116, 57]],
          decision: [[engine - 116, 86], [service + 95, 86]],
          gate: [[service, 104], [service, 137]],
          action: [[service, 159], [service, 192]],
          policy: [[engine - 83, 192], [engine - 83, 104]],
          facts: [[engine + 83, 192], [engine + 83, 104]],
          publish: [[engine + 83, 285], [engine + 83, 254]],
        },
        labels: {
          rar: [(service + engine) / 2 - 10, 36, 'RAR'],
          decision: [(service + engine) / 2 - 5, 108, 'Decision'],
          action: [service + 52, 152, 'Action gate'],
        },
      };
    }

    const left = w * .25;
    const right = w * .75;
    const center = w / 2;
    const margin = w - 3;
    const column = Math.min(150, w / 2 - 14);
    return {
      height: 492,
      gate: [right, 459],
      positions: {
        jwt: [left, 43, column], service: [right, 43, column],
        registry: [left, 121, column], engine: [center, 216, 220],
        policy: [left, 326, column], facts: [right, 326, column],
        publisher: [right, 418, column], action: [left, 448, column],
      },
      links: {
        jwt: [[left + column / 2 + 1, 43], [right - column / 2 - 1, 43]],
        registry: [[left + column / 2 + 1, 121], [right - 45, 121], [right - 45, 74]],
        rar: [[right - 18, 74], [right - 18, 137], [center + 30, 137], [center + 30, 185]],
        decision: [[center + 83, 185], [center + 83, 154], [right + 25, 154], [right + 25, 74]],
        gate: [[right + column / 2 + 1, 43], [margin, 43], [margin, 459], [right + 11, 459]],
        action: [[right - 11, 459], [left + column / 2 + 1, 459]],
        policy: [[left, 295], [left, 270], [center - 62, 270], [center - 62, 248]],
        facts: [[right, 295], [right, 270], [center + 62, 270], [center + 62, 248]],
        publish: [[right, 387], [right, 357]],
      },
      labels: {
        rar: [center + 11, 167, 'RAR'],
        decision: [center + 82, 147, 'Decision'],
        action: [right, 483, 'Action gate'],
      },
    };
  }

  function draw(w) {
    const svg = d3.select('#trace-map');
    const layout = arrange(w);
    svg.attr('viewBox', `0 0 ${w} ${layout.height}`).attr('height', layout.height);
    svg.selectAll('*').remove();
    svg.append('title').attr('id', 'trace-map-title').text('Deployment authorization and policy evaluation');
    svg.append('desc').attr('id', 'trace-map-description').text(
      'The service validates the signed JWT and matches its custom endpoint claim to a configured route. ' +
      'It sends the authenticated Resource-Action-Request to the policy engine. After validation, the engine captures active policy revisions and a scoped snapshot of local facts. ' +
      'An independent publisher updates facts. The service controls the release based on the returned decision.'
    );

    const defs = svg.append('defs');
    for (const [type, style] of Object.entries(palette)) {
      defs.append('marker').attr('id', 'trace-arrow-' + type)
        .attr('viewBox', '0 0 8 8').attr('refX', 7).attr('refY', 4)
        .attr('markerWidth', 6).attr('markerHeight', 6).attr('orient', 'auto')
        .append('path').attr('class', style.route ?? null)
        .attr('d', 'M1,1 L7,4 L1,7').attr('fill', 'none')
        .attr('stroke', style.color).attr('stroke-width', 1.4);
    }

    const paths = new Map();
    const routeLayer = svg.append('g');
    const routeTypes = {
      jwt: 'transport', registry: 'transport', rar: 'transport', decision: 'decision',
      gate: 'decision', action: 'decision', policy: 'policy', facts: 'evidence', publish: 'publish',
    };
    for (const [key, points] of Object.entries(layout.links)) {
      const type = routeTypes[key];
      const style = palette[type];
      const path = routeLayer.append('path').attr('class', 'route ' + (style.route ?? ''))
        .attr('data-link', key).attr('d', d3.line()(points)).attr('stroke', style.color)
        .attr('stroke-dasharray', style.dash ?? null).attr('marker-end', `url(#trace-arrow-${type})`);
      paths.set(key, { path: path.node(), style });
    }

    for (const [x, y, text] of Object.values(layout.labels)) {
      svg.append('text').attr('class', 'edge-label').attr('x', x).attr('y', y)
        .attr('text-anchor', 'middle').text(text);
    }

    const stations = new Map();
    const nodeLayer = svg.append('g');
    const labels = {
      jwt: ['Signed JWT', 'Endpoint claim'],
      registry: ['Trusted routes', 'Configured scope'],
      service: ['Release service', 'Verify token', 'Match route'],
      engine: ['Policy engine', 'Grant · scope · schema', 'Approval rule · expiry recheck'],
      policy: ['Policy catalog', 'Immutable revision r7'],
      facts: ['Fact store', 'Snapshot r42 · TTL'],
      publisher: ['Evidence integration', 'Publish independently'],
      action: ['Release action', 'Service applies decision'],
    };

    for (const [id, [title, ...notes]] of Object.entries(labels)) {
      const [x, y, size] = layout.positions[id];
      const compact = w < 740;
      const route = ({ policy: 'p1', facts: 'p2', publisher: 'p2', engine: 'p3' })[id] ?? '';
      const node = nodeLayer.append('g').attr('class', 'station ' + route)
        .attr('data-station', id).attr('transform', `translate(${x},${y})`);
      node.append('rect').attr('x', -size / 2).attr('y', -30)
        .attr('width', size).attr('height', 60).attr('rx', 6);
      node.append('text').attr('text-anchor', 'middle').attr('y', notes.length > 1 ? -11 : -3)
        .text(compact && id === 'publisher' ? 'Evidence sync' : title);
      if (compact && id === 'action') notes[0] = 'Service applies allow';
      if (compact && id === 'publisher') notes[0] = 'Async publication';
      if (compact && id === 'policy') notes[0] = 'Immutable r7';
      notes.forEach((note, index) => {
        node.append('text').attr('class', 'secondary').attr('text-anchor', 'middle')
          .attr('y', notes.length > 1 ? 7 + index * 15 : 14).text(note);
      });
      stations.set(id, node);
    }

    const [gateX, gateY] = layout.gate;
    const gate = nodeLayer.append('path').attr('class', 'action-gate p3')
      .attr('data-station', 'gate').attr('d', `M${gateX},${gateY - 9} l9,9 -9,9 -9,-9 Z`);
    stations.set('gate', gate);

    return { svg, paths, stations, packets: svg.append('g').attr('aria-hidden', 'true') };
  }

  function reset() {
    generation += 1;
    run.disabled = false;
    if (diagram) {
      diagram.packets.selectAll('*').interrupt().remove();
      diagram.svg.selectAll('.reached,.blocked').classed('reached blocked', false);
    }
    result.textContent = idle;
    delete result.dataset.outcome;
    delete result.dataset.decision;
  }

  function pulse(key) {
    if (reducedMotion.matches) return Promise.resolve(true);
    const { path, style } = diagram.paths.get(key);
    const length = path.getTotalLength();
    const dot = diagram.packets.append('circle').attr('class', 'packet ' + (style.route ?? ''))
      .attr('r', 5).attr('fill', style.color);
    const start = path.getPointAtLength(0);
    dot.attr('cx', start.x).attr('cy', start.y);
    return dot.transition().duration(320).ease(d3.easeLinear)
      .attrTween('transform', () => position => {
        const point = path.getPointAtLength(position * length);
        return `translate(${point.x - start.x},${point.y - start.y})`;
      }).end().then(() => true, () => false).finally(() => dot.remove());
  }

  async function trace(state, active) {
    const reached = (key, status = 'reached') => diagram.stations.get(key).classed(status, true);
    const step = async key => await pulse(key) && active === generation;
    function finish() {
      const [outcome, decision, message] = outcomes[state];
      result.dataset.outcome = outcome;
      result.dataset.decision = decision;
      result.textContent = message;
    }

    reached('jwt');
    result.textContent = 'Verifying JWT…';
    if (!await step('jwt')) return;
    reached('service');
    if (state === 'jwt') {
      reached('service', 'blocked');
      return finish();
    }
    result.textContent = 'Matching the trusted route…';
    reached('registry');
    if (!await step('registry')) return;
    if (state === 'endpoint') {
      reached('registry', 'blocked');
      reached('service', 'blocked');
      return finish();
    }
    result.textContent = 'Sending the evaluation request…';
    if (!await step('rar')) return;
    reached('engine');
    if (state === 'unavailable') {
      reached('engine', 'blocked');
      reached('service', 'blocked');
      reached('gate', 'blocked');
      return finish();
    }
    result.textContent = 'Checking policy and captured facts…';
    reached('policy');
    reached('facts');
    const inputs = await Promise.all([step('policy'), step('facts')]);
    if (!inputs.every(Boolean)) return;
    result.textContent = 'Rechecking fact expiry before the decision…';
    if (state === 'stale') reached('facts', 'blocked');
    if (!await step('decision')) return;
    if (state !== 'allowed') {
      reached('service', 'blocked');
      reached('gate', 'blocked');
      return finish();
    }
    if (!await step('gate')) return;
    reached('gate');
    if (!await step('action')) return;
    reached('action');
    finish();
  }

  run.addEventListener('click', async () => {
    if (!diagram) return;
    reset();
    const active = generation;
    run.disabled = true;
    await trace(scenario.value, active);
    if (active === generation) run.disabled = false;
  });
  scenario.addEventListener('change', reset);
  reducedMotion.addEventListener('change', () => {
    if (run.disabled) reset();
  });

  new ResizeObserver(([{ contentRect }]) => {
    if (contentRect.width === width) return;
    reset();
    width = contentRect.width;
    diagram = draw(width);
  }).observe(canvas);
})();
