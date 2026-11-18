(() => {
  const $ = id => document.getElementById(id);
  // Repository Markdown only: relative paths, optionally climbing out of docs/.
  const allowed = /^(?:\.\.\/)*(?:[\w-]+\/)*[\w.-]+\.md$/;
  const requested = new URLSearchParams(location.search).get('path') || 'documentation-index.md';
  const status = $('doc-status');
  const body = $('doc-body');

  function fail(message) {
    status.textContent = message;
    status.classList.add('error');
  }

  // Path of a same-site URL relative to docs/, the form doc.html?path= expects.
  function fromDocs(url) {
    const base = new URL('./', location.href).pathname.split('/').slice(0, -1);
    const target = url.pathname.split('/');
    let common = 0;
    while (common < base.length && base[common] === target[common]) common++;
    return '../'.repeat(base.length - common) + target.slice(common).join('/');
  }

  function slugger() {
    const seen = new Map();
    return text => {
      const slug = text.toLowerCase().trim().replace(/[^\p{L}\p{N}\s_-]/gu, '').replace(/\s/g, '-');
      const count = seen.get(slug) || 0;
      seen.set(slug, count + 1);
      return count ? `${slug}-${count}` : slug;
    };
  }

  function rewrite(source) {
    for (const link of body.querySelectorAll('a[href]')) {
      const href = link.getAttribute('href');
      if (href.startsWith('#') || /^[a-z][\w+.-]*:/i.test(href)) continue;
      const target = new URL(href, source);
      if (target.origin !== location.origin) continue;
      const project = new URL('../', location.href);
      const path = target.pathname.slice(project.pathname.length);
      // Pages omits dotfiles, including GitHub workflows and repository agent guides.
      if (target.pathname.startsWith(project.pathname) && path.split('/').some(part => part.startsWith('.'))) {
        link.href = `https://github.com/jrepp/ipe/blob/main/${path}${target.hash}`;
      } else {
        link.href = target.pathname.endsWith('.md') ? `doc.html?path=${fromDocs(target)}${target.hash}` : target.href;
      }
    }
    for (const image of body.querySelectorAll('img[src]')) image.src = new URL(image.getAttribute('src'), source).href;
  }

  function outline() {
    const slug = slugger();
    const toc = $('doc-toc');
    const entries = [];
    for (const heading of body.querySelectorAll('h1, h2, h3')) {
      heading.id = heading.id || slug(heading.textContent);
      if (heading.tagName === 'H2') {
        const link = document.createElement('a');
        link.href = `#${heading.id}`;
        link.textContent = heading.textContent;
        entries.push(link);
      }
    }
    if (toc && entries.length > 1) { toc.append(...entries); toc.hidden = false; }
  }

  async function diagrams() {
    const blocks = [...body.querySelectorAll('pre > code.language-mermaid')];
    if (!blocks.length) return '';
    if (!window.mermaid) return ` · ${blocks.length} diagram${blocks.length > 1 ? 's' : ''} shown as source (Mermaid unavailable)`;
    // Mermaid sizes boxes by measuring text, so wait for the font it will draw in.
    await document.fonts?.ready;
    const nodes = blocks.map(code => {
      const diagram = document.createElement('div');
      diagram.className = 'mermaid';
      diagram.textContent = code.textContent;
      code.parentElement.replaceWith(diagram);
      return diagram;
    });
    // Render one at a time with explicit ids so diagrams never share a target.
    let failed = 0;
    for (const [index, node] of nodes.entries()) {
      try {
        const { svg, bindFunctions } = await window.mermaid.render(`doc-diagram-${index}`, node.textContent);
        node.innerHTML = svg;
        bindFunctions?.(node);
      } catch {
        failed++;
        const pre = document.createElement('pre');
        pre.textContent = node.textContent;
        node.replaceWith(pre);
      }
    }
    const count = ` · ${nodes.length} diagram${nodes.length > 1 ? 's' : ''}`;
    return failed ? `${count}, ${failed} shown as source (invalid Mermaid)` : count;
  }

  async function render() {
    $('doc-path').textContent = requested;
    if (!allowed.test(requested)) return fail(`“${requested}” is not a repository Markdown path.`);
    const source = new URL(requested, location.href);
    $('doc-source').href = source.href;
    for (const link of document.querySelectorAll('.sidebar a')) {
      if (link.getAttribute('href') === `doc.html?path=${requested}`) link.setAttribute('aria-current', 'page');
    }
    let text;
    try {
      const response = await fetch(source);
      if (!response.ok) throw new Error();
      text = await response.text();
    } catch {
      return fail(`Could not load ${requested}. Run make docs-site, then serve the packaged site.`);
    }
    if (!window.marked) {
      const pre = document.createElement('pre');
      pre.textContent = text;
      body.replaceChildren(pre);
      return fail('The Markdown renderer did not load, so this is the raw source.');
    }
    // Structured design records use a leading YAML block with plain title and status fields.
    const frontmatter = requested.startsWith('../docs-cms/')
      ? text.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/)
      : null;
    const metadata = frontmatter?.[1] || '';
    const sourceTitle = metadata.match(/^title: *(.+)$/m)?.[1].trim();
    const sourceStatus = metadata.match(/^status: *(.+)$/m)?.[1].trim();
    body.innerHTML = window.marked.parse(text.slice(frontmatter?.[0].length || 0), { gfm: true });
    if (sourceTitle && body.querySelector('h1')?.textContent !== sourceTitle) {
      for (const heading of body.querySelectorAll('h1, h2, h3, h4, h5')) {
        const section = document.createElement(`h${Number(heading.tagName.slice(1)) + 1}`);
        section.append(...heading.childNodes);
        heading.replaceWith(section);
      }
      const heading = document.createElement('h1');
      heading.textContent = sourceTitle;
      body.prepend(heading);
    }
    rewrite(source);
    outline();
    const title = body.querySelector('h1');
    if (title) document.title = `${title.textContent} · IPE`;
    status.textContent = [sourceStatus, `Rendered from ${requested}${await diagrams()}`].filter(Boolean).join(' · ');
    if (location.hash) document.getElementById(decodeURIComponent(location.hash.slice(1)))?.scrollIntoView();
  }

  // Configure before any await: Mermaid's own load handler would otherwise render
  // diagrams with default settings while this script is waiting.
  window.mermaid?.initialize({
    startOnLoad: false, securityLevel: 'strict', theme: 'neutral',
    fontFamily: 'Instrument Sans, system-ui, -apple-system, Segoe UI, sans-serif',
  });
  render();
})();
