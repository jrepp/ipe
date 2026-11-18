(() => {
  const run = document.querySelector('#modeling-run');
  const motion = matchMedia('(prefers-reduced-motion: reduce)');
  const charts = new Map();
  const descriptions = {
    before: 'The agent attempts production release R-42. The release service reads change CHG-42 from the change register. The service rejects the request because it has no approval. The agent cannot approve its own request and has no known reviewer route.',
    after: 'Before requesting production release R-42, the agent consults policy REL-7 revision 3 and control APR-1. The change register reports no approval for CHG-42 at 10:00. The observation expires at 10:05. The control identifies the production release reviewer. The agent requests human review and waits without attempting deployment.',
  };
  const results = new Map(Object.keys(descriptions).map(id => {
    const element = document.querySelector(`#modeling-${id}-result`);
    return [id, { element, summary: element.textContent }];
  }));

  function draw(id, width) {
    const after = id === 'after';
    const svg = d3.select('#modeling-' + id);
    const height = 288;
    const mid = width / 2;
    const spread = Math.min(116, width / 3.1);
    const top = 44;
    const junction = 137;
    const bottom = 233;
    const paths = [];

    svg.attr('viewBox', `0 0 ${width} ${height}`).attr('height', height);
    svg.selectAll('*').remove();
    svg.append('title').attr('id', `modeling-${id}-graph-title`)
      .text(after ? 'Release plan with connected policy and evidence' : 'Release attempt without a policy model');
    svg.append('desc').attr('id', `modeling-${id}-graph-desc`).text(descriptions[id]);

    const routes = svg.append('g').attr('aria-hidden', 'true');
    function route(points, tone, arrival) {
      const path = routes.append('path').attr('class', `modeling-route ${tone}`)
        .attr('d', d3.line()(points));
      paths.push({ path: path.node(), tone, arrival });
    }

    const nodes = svg.append('g').attr('aria-hidden', 'true');
    function station(key, x, y, label, glyph, tone, shape) {
      const group = nodes.append('g').attr('class', `modeling-station ${tone}`)
        .attr('data-node', key).attr('transform', `translate(${x},${y})`);
      if (shape === 'diamond') group.append('path').attr('class', 'station-mark').attr('d', 'M0,-17 17,0 0,17 -17,0 Z');
      else group.append('circle').attr('class', 'station-mark').attr('r', 16);
      group.append('text').attr('class', 'station-glyph').attr('y', 1).text(glyph);
      group.append('text').attr('class', 'station-label').attr('y', 33).text(label);
      return group;
    }

    station('agent', mid, top, 'Agent', 'A', 'agent');
    route([[mid, top + 16], [mid, junction]], 'agent', after ? null : 'gate');
    if (after) {
      route([[mid - spread, junction], [mid, junction]], 'p1');
      route([[mid + spread, junction], [mid, junction]], 'p2');
      route([[mid, junction], [mid, bottom - 16]], 'handoff', 'handoff');
      station('policy', mid - spread, junction, 'Policy + controls', 'P1', 'p1');
      station('evidence', mid + spread, junction, 'Evidence', 'P2', 'p2');
      nodes.append('circle').attr('class', 'modeling-junction').attr('cx', mid).attr('cy', junction).attr('r', 6);
      station('handoff', mid, bottom, 'Human review', '', 'handoff')
        .append('path').attr('class', 'station-icon').attr('d', 'M0,-8 V0 L5,3');
      nodes.append('text').attr('class', 'station-note').attr('x', mid).attr('y', junction + 28)
        .text('Approval missing');
    } else {
      route([[mid, junction], [mid + spread, junction]], 'p2 source', 'source');
      route([[mid + spread, junction], [mid, junction]], 'p2 source');
      route([[mid, junction], [mid, bottom - 16]], 'rejection', 'error');
      station('gate', mid, junction, 'Approval check', '?', 'rejection', 'diamond');
      station('source', mid + spread, junction, 'Change register', 'S', 'p2');
      station('error', mid, bottom, 'Release rejected', '×', 'rejection');
    }

    charts.set(id, {
      width,
      paths,
      nodes,
      packets: svg.append('g').attr('aria-hidden', 'true'),
    });
  }

  let active = false;
  let generation = 0;

  function clear() {
    generation += 1;
    active = false;
    run.disabled = false;
    for (const [id, chart] of charts) {
      chart.packets.selectAll('*').interrupt().remove();
      chart.nodes.selectAll('.visited').classed('visited', false);
      const { element, summary } = results.get(id);
      element.textContent = summary;
      delete element.dataset.outcome;
    }
  }

  function finish(id) {
    charts.get(id).nodes.selectAll('[data-node]').classed('visited', true);
    const { element, summary } = results.get(id);
    element.textContent = summary;
    element.dataset.outcome = id === 'before' ? 'blocked' : 'pending';
  }

  async function trace(id, token) {
    const { paths, packets, nodes } = charts.get(id);
    nodes.select('[data-node="agent"]').classed('visited', true);
    if (!motion.matches) {
      for (const { path, tone, arrival } of paths) {
        const length = path.getTotalLength();
        const start = path.getPointAtLength(0);
        if (tone === 'p1') nodes.select('[data-node="policy"]').classed('visited', true);
        if (tone === 'p2') nodes.select('[data-node="evidence"]').classed('visited', true);
        const packet = packets.append('circle').attr('class', `packet modeling-packet ${tone}`)
          .attr('r', 6).attr('transform', `translate(${start.x},${start.y})`);
        try {
          await packet.transition().duration(460).ease(d3.easeCubicInOut)
            .attrTween('transform', () => progress => {
              const point = path.getPointAtLength(progress * length);
              return `translate(${point.x},${point.y})`;
            }).end();
        } catch {
          return;
        }
        if (token !== generation) return;
        packet.remove();
        if (arrival) nodes.select(`[data-node="${arrival}"]`).classed('visited', true);
      }
    }
    finish(id);
  }

  run.addEventListener('click', async () => {
    if (active || charts.size !== results.size) return;
    clear();
    active = true;
    run.disabled = true;
    const token = generation;
    for (const { element } of results.values()) element.textContent = 'Tracing release planning.';
    await Promise.all([...charts.keys()].map(id => trace(id, token)));
    if (token !== generation) return;
    active = false;
    run.disabled = false;
  });

  motion.addEventListener('change', () => {
    if (!active || !motion.matches) return;
    clear();
    for (const id of charts.keys()) finish(id);
  });

  const observer = new ResizeObserver(entries => {
    const changes = entries.map(({ target, contentRect }) => ({
      id: target.querySelector('svg').id.replace('modeling-', ''),
      width: Math.floor(contentRect.width),
    })).filter(({ id, width }) => width > 0 && charts.get(id)?.width !== width);
    if (changes.length === 0) return;
    if (active) clear();
    for (const { id, width } of changes) {
      draw(id, width);
      if (results.get(id).element.dataset.outcome) finish(id);
    }
  });
  document.querySelectorAll('.modeling .canvas').forEach(canvas => observer.observe(canvas));
})();
