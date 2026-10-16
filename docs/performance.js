(() => {
  const $ = id => document.getElementById(id);
  const number = value => value.toLocaleString(undefined, { maximumFractionDigits: 3 });
  const ns = 'http://www.w3.org/2000/svg';
  function svg(tag, attributes, content) {
    const element = document.createElementNS(ns, tag);
    Object.entries(attributes).forEach(([key, value]) => element.setAttribute(key, value));
    if (content !== undefined) element.textContent = content;
    return element;
  }
  let data;
  function render() {
    const metric = $('percentile').value;
    const chosen = data.workloads[Number($('workload').value)];
    $('latency-label').textContent = `${metric.split('_')[0]} latency`;
    $('latency').textContent = number(chosen[metric]);
    $('throughput').textContent = Math.round(chosen.throughput).toLocaleString();
    $('samples').textContent = chosen.samples.toLocaleString();
    $('outliers').textContent = chosen.outliers.toLocaleString();
    $('outlier-note').textContent = `above ${number(chosen.outlier_threshold_us)} µs`;
    const latency = $('latency-chart'); latency.replaceChildren();
    const max = Math.max(...data.workloads.map(w => w[metric]), 0.001) * 1.2;
    for (let tick = 0; tick <= 4; tick++) {
      const x = 130 + tick * 90;
      latency.append(svg('line', { x1:x, y1:16, x2:x, y2:200, stroke:'#e5e8f1' }), svg('text', { x, y:225, 'text-anchor':'middle' }, number(max*tick/4)));
    }
    data.workloads.forEach((w, i) => {
      const y = 30+i*58;
      const bar = svg('rect', { x:130, y, width:w[metric]/max*360, height:28, rx:4, class:'bar' });
      bar.append(svg('title', {}, `${w.name}: ${number(w[metric])} µs`));
      latency.append(svg('text', { x:120, y:y+19, 'text-anchor':'end' }, w.name), bar, svg('text', { x:136+w[metric]/max*360, y:y+19 }, number(w[metric])));
    });
    const distribution = $('distribution-chart'); distribution.replaceChildren();
    const peak = Math.max(chosen.max_us, 0.001)*1.1;
    for (let tick = 0; tick <= 4; tick++) {
      const y = 195-tick*42;
      distribution.append(svg('line', { x1:65, x2:510, y1:y, y2:y, stroke:'#e5e8f1' }), svg('text', { x:57, y:y+4, 'text-anchor':'end' }, number(peak*tick/4)));
    }
    for (const p of [1,25,50,75,100]) distribution.append(svg('text', { x:65+(p-1)/99*445, y:225, 'text-anchor':'middle' }, `p${p}`));
    const points = chosen.quantiles.map(q => `${65+(q.percentile-1)/99*445},${195-q.us/peak*168}`).join(' ');
    distribution.append(svg('polyline', { points, fill:'none', stroke:'#6851b4', 'stroke-width':2.5 }));
    for (const q of chosen.quantiles) {
      const dot = svg('circle', { cx:65+(q.percentile-1)/99*445, cy:195-q.us/peak*168, r:2, fill:'#6851b4' });
      dot.append(svg('title', {}, `p${q.percentile}: ${number(q.us)} µs`)); distribution.append(dot);
    }
    $('measurements').replaceChildren(...data.workloads.map(w => {
      const tr = document.createElement('tr'); if (w === chosen) tr.className = 'selected';
      [w.name,w.instructions,number(w.p50_us),number(w.p95_us),number(w.p99_us),number(w.max_us),Math.round(w.throughput).toLocaleString()].forEach(value => { const td = document.createElement('td'); td.textContent = value; tr.append(td); }); return tr;
    }));
    const passing = data.workloads.filter(w => w.p99_us < 50).length;
    $('latency-target').textContent = `${passing}/${data.workloads.length} local fixtures below target`;
    $('throughput-target').textContent = `${data.workloads.filter(w => w.throughput >= 100000).length}/${data.workloads.length} local fixtures reach target (thread not pinned)`;
  }
  fetch('data/site-data.json').then(response => { if (!response.ok) throw new Error('No measurement file found.'); return response.json(); }).then(result => {
    if (result.schema_version !== 1 || !result.workloads?.length) throw new Error('Unsupported measurement data.');
    data = result;
    $('workload').replaceChildren(...data.workloads.map((w,i) => new Option(w.name,i)));
    $('workload').addEventListener('change', render); $('percentile').addEventListener('change', render);
    $('perf-status').textContent = `Measured ${new Date(data.generated_at).toLocaleString()} · ${data.platform} · ${data.revision}${data.working_tree_modified ? ' + local changes' : ''}`;
    $('provenance').textContent = `${data.rustc} · ${data.profile} profile · ${data.engine} · ${data.platform}`;
    render(); $('perf-content').hidden = false;
  }).catch(error => { $('perf-status').textContent = `${error.message} Run make docs-site to generate local measurements.`; $('perf-status').classList.add('error'); });
})();
