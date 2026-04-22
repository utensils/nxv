/* nxv — front-end logic, wired to the live /api/v1 endpoints */
(() => {
  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => [...r.querySelectorAll(s)];

  const API_BASE = '/api/v1';
  const PAGE_SIZE = 50;
  const FLAKES_EPOCH = new Date('2020-02-10T00:00:00Z');

  const STATE = {
    query: '',
    filters: {
      exact: false,
      version: '',
      arch: '',
      license: '',
      sort: 'date',
      includeInsecure: false,
    },
    view: 'rows',
    page: 1,
    pageSize: PAGE_SIZE,
    total: 0,
    hasMore: false,
    lastLatencyMs: null,
    stats: null,
    health: null,
    reqSeq: 0,
    historyCache: new Map(),
    firstHashCache: new Map(),
  };

  const els = {};
  const cache = (id) => (els[id] = els[id] || document.getElementById(id));

  // ---------- util ----------
  const fmtDate = (d) => {
    if (!d) return '—';
    try {
      return new Date(d).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: '2-digit',
      });
    } catch {
      return String(d);
    }
  };
  const fmtNum = (n) => (n == null ? '—' : Number(n).toLocaleString());
  const archLabel = (p) =>
    p
      .replace('x86_64-linux', 'x86_64·linux')
      .replace('aarch64-linux', 'aarch64·linux')
      .replace('x86_64-darwin', 'x86_64·macos')
      .replace('aarch64-darwin', 'aarch64·macos')
      .replace('i686-linux', 'i686·linux')
      .replace('armv7l-linux', 'armv7·linux');
  const predatesFlakes = (d) => (d ? new Date(d) < FLAKES_EPOCH : false);
  const shortHash = (h) => (h || '').slice(0, 7);
  const escapeHtml = (s) =>
    String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));

  function parseJsonArrayOrString(s) {
    if (s == null || s === '') return [];
    if (typeof s !== 'string') return Array.isArray(s) ? s : [String(s)];
    const t = s.trim();
    if (t.startsWith('[')) {
      try {
        const v = JSON.parse(t);
        return Array.isArray(v) ? v : [String(v)];
      } catch {
        return [s];
      }
    }
    return [s];
  }

  function toRow(p) {
    const licenses = parseJsonArrayOrString(p.license);
    const platforms = parseJsonArrayOrString(p.platforms).filter(Boolean);
    const vulns = parseJsonArrayOrString(p.known_vulnerabilities);
    return {
      id: p.id,
      name: p.name,
      attr: p.attribute_path,
      ver: p.version,
      desc: p.description || '',
      first: p.first_commit_date,
      last: p.last_commit_date,
      hash: p.first_commit_hash,
      lastHash: p.last_commit_hash,
      platforms,
      license: licenses.join(' / ') || '',
      insecure: vulns.length ? vulns : null,
      legacy: predatesFlakes(p.last_commit_date),
      homepage: p.homepage || null,
      sourcePath: p.source_path || null,
    };
  }

  async function api(path) {
    const t0 = performance.now();
    const res = await fetch(path, { headers: { Accept: 'application/json' } });
    const latency = performance.now() - t0;
    if (!res.ok) {
      const err = new Error(`${res.status} ${res.statusText}`);
      err.status = res.status;
      throw err;
    }
    const json = await res.json();
    return { json, latency };
  }

  // ---------- toast / copy ----------
  function showToast(msg) {
    const t = cache('toast');
    t.textContent = msg;
    t.style.opacity = '1';
    t.style.transform = 'translateY(0)';
    clearTimeout(t._t);
    t._t = setTimeout(() => {
      t.style.opacity = '0';
      t.style.transform = 'translateY(1rem)';
    }, 1800);
  }
  async function copy(text) {
    try {
      await navigator.clipboard.writeText(text);
      showToast(`copied · ${text.length > 56 ? text.slice(0, 56) + '…' : text}`);
    } catch {
      showToast('copy failed');
    }
  }

  function buildFlakeCmd(r) {
    const isLegacy = r.legacy || predatesFlakes(r.last);
    const insecurePrefix = r.insecure ? 'NIXPKGS_ALLOW_INSECURE=1 ' : '';
    const impure = r.insecure ? ' --impure' : '';
    const ref = shortHash(r.hash || r.lastHash);
    return isLegacy
      ? `${insecurePrefix}nix-shell -p '(import (builtins.fetchTarball "https://github.com/NixOS/nixpkgs/archive/${ref}.tar.gz") {}).${r.attr}'`
      : `${insecurePrefix}nix shell${impure} nixpkgs/${ref}#${r.attr}`;
  }

  // ---------- URL state (refresh-safe, shareable) ----------
  function serializeState() {
    const p = new URLSearchParams();
    if (STATE.query) p.set('q', STATE.query);
    if (STATE.filters.exact) p.set('exact', '1');
    if (STATE.filters.version) p.set('version', STATE.filters.version);
    if (STATE.filters.arch) p.set('arch', STATE.filters.arch);
    if (STATE.filters.license) p.set('license', STATE.filters.license);
    if (STATE.filters.sort && STATE.filters.sort !== 'date') p.set('sort', STATE.filters.sort);
    if (STATE.filters.includeInsecure) p.set('insecure', '1');
    if (STATE.view && STATE.view !== 'rows') p.set('view', STATE.view);
    if (STATE.page > 1) p.set('page', String(STATE.page));
    return p;
  }

  function syncUrl() {
    const qs = serializeState().toString();
    const url = qs ? `${window.location.pathname}?${qs}` : window.location.pathname;
    // replaceState so every keystroke doesn't bloat browser history
    window.history.replaceState(null, '', url);
  }

  function hydrateFromUrl() {
    const p = new URLSearchParams(window.location.search);
    STATE.query = p.get('q') || '';
    STATE.filters.exact = p.get('exact') === '1';
    STATE.filters.version = p.get('version') || '';
    STATE.filters.arch = p.get('arch') || '';
    STATE.filters.license = p.get('license') || '';
    STATE.filters.sort = p.get('sort') || 'date';
    STATE.filters.includeInsecure = p.get('insecure') === '1';
    STATE.view = p.get('view') || 'rows';
    const pg = parseInt(p.get('page') || '1', 10);
    STATE.page = Number.isFinite(pg) && pg > 0 ? pg : 1;
  }

  // ---------- query parsing ----------
  function parseQuery(q) {
    const m = q.trim().match(/^(.+?)\s+(v?\d[\d.]*[\w.-]*)$/i);
    if (m && !m[1].match(/\d$/)) return { pkg: m[1].trim(), ver: m[2].replace(/^v/i, '') };
    return { pkg: q.trim(), ver: null };
  }

  // ---------- filter chips ----------
  function cycleFilter(key) {
    const cycles = {
      exact: [false, true],
      version: ['', '2.7', '3.11', '3.12', '18', '22'],
      arch: ['', 'x86_64-linux', 'aarch64-linux', 'x86_64-darwin', 'aarch64-darwin'],
      license: ['', 'MIT', 'GPL-3.0+', 'BSD-3-Clause', 'Apache-2.0', 'LGPL-2.1+'],
      sort: ['date', 'name', 'version'],
      includeInsecure: [false, true],
    };
    const cycle = cycles[key];
    const cur = STATE.filters[key];
    const idx = cycle.indexOf(cur);
    STATE.filters[key] = cycle[(idx + 1) % cycle.length];
  }

  function renderFilterChips() {
    const labels = {
      exact: () => (STATE.filters.exact ? 'on' : 'off'),
      version: () => STATE.filters.version || 'any',
      arch: () => (STATE.filters.arch ? archLabel(STATE.filters.arch) : 'any'),
      license: () => STATE.filters.license || 'any',
      sort: () => STATE.filters.sort,
      'include-insecure': () => (STATE.filters.includeInsecure ? 'yes' : 'no'),
    };
    $$('.chip[data-filter]').forEach((el) => {
      const k = el.dataset.filter;
      if (k === 'include-insecure') {
        el.innerHTML = `include insecure: <span class="ml-1 text-[var(--color-fog-0)]">${labels[k]()}</span>`;
        el.classList.toggle('active', STATE.filters.includeInsecure);
      } else {
        const label = el.textContent.split(':')[0].trim();
        el.innerHTML = `${label}: <span class="ml-1 text-[var(--color-fog-0)]">${labels[k]()}</span>`;
        const v = STATE.filters[k];
        const isActive =
          k === 'exact'
            ? STATE.filters.exact
            : k === 'sort'
            ? v !== 'date'
            : !!v;
        el.classList.toggle('active', !!isActive);
      }
    });
  }

  // ---------- search ----------
  function buildSearchUrl() {
    const parsed = parseQuery(STATE.query);
    const pkg = parsed.pkg;
    const params = new URLSearchParams();
    // API requires q — use " " as no-op to fetch top rows
    params.set('q', pkg || '');
    const ver = STATE.filters.version || parsed.ver || '';
    if (ver) params.set('version', ver);
    if (STATE.filters.exact) params.set('exact', 'true');
    if (STATE.filters.license) params.set('license', STATE.filters.license);
    if (STATE.filters.sort) params.set('sort', STATE.filters.sort);
    params.set('limit', String(STATE.pageSize));
    params.set('offset', String((STATE.page - 1) * STATE.pageSize));
    return `${API_BASE}/search?${params.toString()}`;
  }

  async function runSearch(opts = {}) {
    if (opts.resetPage !== false) STATE.page = 1;
    syncUrl();
    const seq = ++STATE.reqSeq;

    // empty query + no filters → show welcome
    const noQuery = !STATE.query.trim();
    const noFilters =
      !STATE.filters.version && !STATE.filters.arch && !STATE.filters.license;
    if (noQuery && noFilters) {
      renderWelcome();
      return;
    }

    const url = buildSearchUrl();
    setResultsStatus('running…', '');
    try {
      const { json, latency } = await api(url);
      if (seq !== STATE.reqSeq) return; // a newer request started — drop this
      STATE.lastLatencyMs = latency;
      const items = (json.data || []).map(toRow);
      const meta = json.meta || { total: items.length, has_more: false };
      STATE.total = meta.total;
      STATE.hasMore = !!meta.has_more;

      // client-side filters that the API doesn't support: arch, includeInsecure
      let filtered = items;
      if (STATE.filters.arch) filtered = filtered.filter((r) => r.platforms.includes(STATE.filters.arch));
      if (!STATE.filters.includeInsecure) filtered = filtered.filter((r) => !r.insecure);

      render(filtered);
      setResultsStatus(
        `results / ${fmtNum(meta.total)}${filtered.length !== items.length ? ` (${filtered.length} shown)` : ''}`,
        `${(latency / 1000).toFixed(3)}s · api`,
      );
      renderPagination(meta);
    } catch (e) {
      if (seq !== STATE.reqSeq) return;
      renderError(e);
    }
  }

  function setResultsStatus(count, time) {
    cache('resultsCount').textContent = count || '—';
    cache('resultsTime').textContent = time || '—';
  }

  function renderWelcome() {
    STATE.total = 0;
    STATE.hasMore = false;
    cache('resultsBody').innerHTML = `
      <div class="px-6 py-16 text-center">
        <div class="mono text-[12px] text-[var(--color-fog-4)] leading-7">
          type a package name above to search<br/>
          <span class="text-[var(--color-fog-3)]">try</span>
          <button class="chip example mx-1" type="button">python 2.7</button>
          <button class="chip example mx-1" type="button">nodejs 18</button>
          <button class="chip example mx-1" type="button">gcc 4.9</button>
          <button class="chip example mx-1" type="button">ffmpeg 5</button>
          <br/>
          <span class="text-[var(--color-fog-3)]">or press</span> <span class="kbd">⌘K</span> <span class="text-[var(--color-fog-3)]">to open the palette</span>
        </div>
      </div>`;
    setResultsStatus('results / —', '—');
    renderPagination({ total: 0, has_more: false });
    // rewire welcome example chips
    $$('#resultsBody .chip.example').forEach((el) =>
      el.addEventListener('click', () => runExample(el.textContent.trim())),
    );
  }

  function renderError(e) {
    cache('resultsBody').innerHTML = `
      <div class="px-6 py-12 text-center">
        <div class="mono text-[12px] text-[var(--color-red-glow)]">error · ${escapeHtml(e?.message || 'request failed')}</div>
        <div class="mt-2 mono text-[11px] text-[var(--color-fog-4)]">check that the API server is reachable.</div>
      </div>`;
    setResultsStatus('results / —', 'error');
    renderPagination({ total: 0, has_more: false });
  }

  function render(rows) {
    const body = cache('resultsBody');
    if (!rows.length) {
      body.innerHTML = `
        <div class="px-6 py-16 text-center">
          <div class="mono text-[12px] text-[var(--color-fog-4)]">no results — try loosening filters, or press <span class="kbd">⌘K</span> to browse.</div>
        </div>`;
      return;
    }
    body.innerHTML = rows.map((r, i) => renderRow(r, i)).join('');

    const rowByAttrVer = new Map(rows.map((r) => [`${r.attr}::${r.ver}`, r]));
    $$('#resultsBody [data-action]').forEach((el) => {
      el.addEventListener('click', (ev) => {
        ev.stopPropagation();
        const { action } = el.dataset;
        const key = el.dataset.key;
        const r = rowByAttrVer.get(key);
        if (!r) return;
        if (action === 'copy-flake') copy(buildFlakeCmd(r));
        else if (action === 'copy-run') copy(`nix run nixpkgs/${shortHash(r.hash || r.lastHash)}#${r.attr}`);
        else if (action === 'history') openDrawer(r);
      });
    });
    $$('#resultsBody [data-row]').forEach((el) => {
      el.addEventListener('click', () => {
        const r = rowByAttrVer.get(el.dataset.row);
        if (r) openDrawer(r);
      });
    });
  }

  function renderRow(r, i) {
    const isLegacy = r.legacy;
    const flags = [];
    if (r.insecure) {
      const title = escapeHtml(r.insecure.join(' · '));
      flags.push(`<span class="chip danger" title="${title}"><svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><path d="M12 8v4M12 16h.01"/></svg>insecure</span>`);
    }
    if (isLegacy) flags.push(`<span class="chip warn">pre-flakes</span>`);

    const platformsHtml = r.platforms
      .filter((p) => /^(x86_64|aarch64|i686|armv7l|armv6l)-(linux|darwin)$/.test(p))
      .slice(0, 4)
      .map((p) => {
        const active = STATE.filters.arch === p;
        return `<span class="chip${active ? ' active' : ''}" style="font-size:10px; padding: 1px 6px;">${archLabel(p)}</span>`;
      })
      .join('');

    const key = `${r.attr}::${r.ver}`;
    const nameHtml = escapeHtml(r.name);
    const attrHtml = escapeHtml(r.attr);
    const licenseHtml = escapeHtml(r.license || '—');
    const descHtml = escapeHtml(r.desc);
    const verHtml = escapeHtml(r.ver);

    return `
      <div data-row="${escapeHtml(key)}" class="group grid grid-cols-[minmax(180px,1.6fr)_100px_minmax(200px,2fr)_120px_130px_90px] gap-3 items-center px-4 py-3 cursor-pointer transition anim-in hover:bg-[var(--color-ink-2)]" style="animation-delay:${i * 12}ms; border-bottom: 1px solid var(--color-ink-2);">
        <div class="min-w-0">
          <div class="flex items-center gap-2">
            <span class="mono text-[13.5px] text-[var(--color-fog-0)] font-medium truncate">${nameHtml}</span>
            <span class="mono text-[11px] text-[var(--color-fog-4)]">·</span>
            <span class="mono text-[11px] text-[var(--color-nix-400)] truncate">${attrHtml}</span>
          </div>
          <div class="mono text-[10.5px] text-[var(--color-fog-4)] mt-0.5 flex items-center gap-1.5">
            <span class="truncate" style="max-width: 180px;">${licenseHtml}</span>
            <span class="text-[var(--color-ink-4)]">·</span>
            <span class="mono">#${shortHash(r.hash || r.lastHash)}</span>
          </div>
        </div>
        <div>
          <span class="mono text-[13px] ${r.insecure ? 'text-[var(--color-red-glow)]' : 'text-[var(--color-fog-0)]'} tabular-nums">${verHtml}</span>
        </div>
        <div class="hidden md:block min-w-0">
          <div class="text-[13px] text-[var(--color-fog-2)] truncate">${descHtml || '<span class="text-[var(--color-fog-4)]">—</span>'}</div>
          <div class="mt-1 flex flex-wrap gap-1">
            ${flags.join('')}${platformsHtml}
          </div>
        </div>
        <div class="mono text-[11.5px] text-[var(--color-fog-3)] tabular-nums">${fmtDate(r.first)}</div>
        <div class="mono text-[11.5px] text-[var(--color-fog-3)] tabular-nums">${fmtDate(r.last)}</div>
        <div class="flex items-center justify-end gap-1 opacity-70 group-hover:opacity-100 transition">
          <button class="btn btn-ghost" data-action="copy-flake" data-key="${escapeHtml(key)}" title="copy flake ref">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
            cp
          </button>
          <button class="btn btn-ghost" data-action="history" data-key="${escapeHtml(key)}" title="version history">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
            log
          </button>
        </div>
      </div>
    `;
  }

  // ---------- pagination ----------
  function renderPagination(meta) {
    const el = cache('pagination');
    const total = meta.total || 0;
    const totalPages = Math.max(1, Math.ceil(total / STATE.pageSize));
    const startIdx = total === 0 ? 0 : (STATE.page - 1) * STATE.pageSize + 1;
    const endIdx = Math.min(total, STATE.page * STATE.pageSize);
    el.innerHTML = `
      <span>page <span class="text-[var(--color-fog-0)]">${STATE.page}</span> / <span class="text-[var(--color-fog-0)]">${totalPages}</span> · showing <span class="text-[var(--color-fog-0)]">${startIdx}–${endIdx}</span> of <span class="text-[var(--color-fog-0)]">${fmtNum(total)}</span></span>
      <div class="flex items-center gap-1.5">
        <button class="btn btn-ghost" data-page="prev" ${STATE.page <= 1 ? 'disabled style="opacity:.4; cursor:not-allowed;"' : ''}>← prev</button>
        <button class="btn btn-ghost" data-page="next" ${!meta.has_more ? 'disabled style="opacity:.4; cursor:not-allowed;"' : ''}>next →</button>
      </div>`;
    $$('#pagination [data-page]').forEach((b) => {
      b.addEventListener('click', () => {
        if (b.hasAttribute('disabled')) return;
        if (b.dataset.page === 'prev' && STATE.page > 1) STATE.page -= 1;
        else if (b.dataset.page === 'next' && STATE.hasMore) STATE.page += 1;
        else return;
        runSearch({ resetPage: false });
        window.scrollTo({ top: cache('resultsSection').offsetTop - 80, behavior: 'smooth' });
      });
    });
  }

  // ---------- drawer / version history ----------
  async function openDrawer(r) {
    const drawer = cache('drawerOverlay');
    drawer.classList.remove('hidden');
    drawer.classList.add('flex');
    cache('drawerTitle').textContent = `${r.name} · ${r.attr}`;
    cache('drawerSub').innerHTML = `${escapeHtml(r.desc || '—')} <span class="text-[var(--color-ink-4)]">│</span> ${escapeHtml(r.license || '—')}`;
    cache('drawerCount').textContent = '…';
    cache('drawerList').innerHTML = `<li class="px-3 py-4 mono text-[12px] text-[var(--color-fog-4)]">loading version history…</li>`;
    cache('timelineViz').innerHTML = '';

    try {
      const versions = await fetchHistory(r.attr);
      cache('drawerCount').textContent = `(${versions.length})`;
      drawTimeline(versions);
      cache('drawerList').innerHTML = versions
        .map((v, idx) => {
          const legacy = predatesFlakes(v.last_seen);
          const tag = v.is_insecure
            ? ' text-[var(--color-red-glow)]'
            : legacy
            ? ' text-[var(--color-amber-glow)]'
            : ' text-[var(--color-fog-0)]';
          return `
            <li class="grid grid-cols-[minmax(90px,auto)_1fr_auto] items-center gap-4 px-3 py-2 rounded-[2px] hover:bg-[var(--color-ink-2)] transition">
              <span class="mono text-[12.5px]${tag} tabular-nums">${escapeHtml(v.version)}</span>
              <span class="mono text-[11px] text-[var(--color-fog-3)] tabular-nums">${fmtDate(v.first_seen)}<span class="text-[var(--color-ink-4)] mx-2">→</span>${fmtDate(v.last_seen)}</span>
              <span class="flex items-center gap-2">
                ${v.is_insecure ? '<span class="chip danger" style="font-size:10px; padding:1px 5px;">insecure</span>' : ''}
                ${legacy ? '<span class="chip warn" style="font-size:10px; padding:1px 5px;">pre-flakes</span>' : ''}
                <button class="btn btn-ghost" data-history-copy="${idx}" title="copy flake ref">cp</button>
              </span>
            </li>`;
        })
        .join('');

      $$('#drawerList [data-history-copy]').forEach((b) => {
        b.addEventListener('click', async () => {
          const v = versions[parseInt(b.dataset.historyCopy, 10)];
          if (!v) return;
          b.textContent = '…';
          try {
            const hash = await fetchFirstHash(r.attr, v.version);
            const synth = {
              attr: r.attr,
              hash,
              last: v.last_seen,
              insecure: v.is_insecure ? ['insecure'] : null,
              legacy: predatesFlakes(v.last_seen),
            };
            copy(buildFlakeCmd(synth));
          } catch (e) {
            showToast(`error · ${e.message}`);
          } finally {
            b.textContent = 'cp';
          }
        });
      });
    } catch (e) {
      cache('drawerCount').textContent = '';
      cache('drawerList').innerHTML = `
        <li class="px-3 py-4 mono text-[12px] text-[var(--color-red-glow)]">error · ${escapeHtml(e.message)}</li>`;
    }
  }

  function closeDrawer() {
    const drawer = cache('drawerOverlay');
    drawer.classList.add('hidden');
    drawer.classList.remove('flex');
  }

  async function fetchHistory(attr) {
    if (STATE.historyCache.has(attr)) return STATE.historyCache.get(attr);
    const { json } = await api(`${API_BASE}/packages/${encodeURIComponent(attr)}/history?limit=100`);
    const versions = (json.data || []).slice();
    STATE.historyCache.set(attr, versions);
    return versions;
  }

  async function fetchFirstHash(attr, version) {
    const key = `${attr}::${version}`;
    if (STATE.firstHashCache.has(key)) return STATE.firstHashCache.get(key);
    const { json } = await api(
      `${API_BASE}/packages/${encodeURIComponent(attr)}/versions/${encodeURIComponent(version)}/first`,
    );
    const hash = json?.data?.first_commit_hash || '';
    STATE.firstHashCache.set(key, hash);
    return hash;
  }

  function drawTimeline(history) {
    const el = cache('timelineViz');
    el.innerHTML = '';

    // Derive axis from the actual history, not a hard-coded 2017 floor.
    const times = [];
    for (const v of history) {
      const f = new Date(v.first_seen).getTime();
      const l = new Date(v.last_seen).getTime();
      if (Number.isFinite(f)) times.push(f);
      if (Number.isFinite(l)) times.push(l);
    }
    if (!times.length) return;

    const rawStart = Math.min(...times);
    const rawEnd = Math.max(...times);
    const minSpan = 90 * 24 * 3600e3; // keep a 3-month floor so single-version histories stay legible
    const usableSpan = Math.max(rawEnd - rawStart, minSpan);
    const pad = Math.max(14 * 24 * 3600e3, usableSpan * 0.04);
    const axisStart = rawStart - pad;
    const axisEnd = rawEnd + pad;
    const axisSpan = axisEnd - axisStart;

    const startYear = new Date(axisStart).getUTCFullYear();
    const endYear = new Date(axisEnd).getUTCFullYear();

    const labelEl = document.getElementById('timelineLabel');
    if (labelEl) labelEl.textContent = `timeline · ${startYear} → ${endYear}`;

    const ticksEl = document.getElementById('timelineTicks');
    if (ticksEl) {
      const years = [];
      for (let y = startYear; y <= endYear; y++) years.push(y);
      // thin ticks to keep the row readable on long histories
      const maxTicks = 10;
      const stride = Math.max(1, Math.ceil(years.length / maxTicks));
      const shown = years.filter((_, i) => i % stride === 0);
      if (shown[shown.length - 1] !== years[years.length - 1]) shown.push(years[years.length - 1]);
      ticksEl.innerHTML = shown
        .map((y) => `<span>'${String(y).slice(2)}</span>`)
        .join('');
    }

    const rows = history.slice(0, 12);
    const rowH = 10;
    const gap = 2;
    const totalH = Math.max(rows.length * (rowH + gap), 100);
    const svgNS = 'http://www.w3.org/2000/svg';
    const svg = document.createElementNS(svgNS, 'svg');
    svg.setAttribute('viewBox', `0 0 1000 ${totalH}`);
    svg.setAttribute('preserveAspectRatio', 'none');
    svg.style.width = '100%';
    svg.style.height = `${totalH}px`;

    for (let y = startYear; y <= endYear; y++) {
      const x = ((new Date(Date.UTC(y, 0, 1)).getTime()) - axisStart) / axisSpan * 1000;
      if (x < 0 || x > 1000) continue;
      const line = document.createElementNS(svgNS, 'line');
      line.setAttribute('x1', x);
      line.setAttribute('x2', x);
      line.setAttribute('y1', 0);
      line.setAttribute('y2', totalH);
      line.setAttribute('stroke', 'var(--color-ink-3)');
      line.setAttribute('stroke-dasharray', '2 4');
      line.setAttribute('stroke-width', '1');
      svg.appendChild(line);
    }

    const flakeT = FLAKES_EPOCH.getTime();
    if (flakeT >= axisStart && flakeT <= axisEnd) {
      const flakeX = ((flakeT - axisStart) / axisSpan) * 1000;
      const flakeLine = document.createElementNS(svgNS, 'line');
      flakeLine.setAttribute('x1', flakeX);
      flakeLine.setAttribute('x2', flakeX);
      flakeLine.setAttribute('y1', 0);
      flakeLine.setAttribute('y2', totalH);
      flakeLine.setAttribute('stroke', 'var(--color-amber-glow)');
      flakeLine.setAttribute('stroke-dasharray', '3 3');
      flakeLine.setAttribute('stroke-width', '1');
      flakeLine.setAttribute('opacity', '0.5');
      svg.appendChild(flakeLine);
    }

    rows.forEach((v, i) => {
      const x1 = Math.max(0, (new Date(v.first_seen).getTime() - axisStart) / axisSpan * 1000);
      const x2 = Math.min(1000, (new Date(v.last_seen).getTime() - axisStart) / axisSpan * 1000);
      const w = Math.max(2, x2 - x1);
      const y = i * (rowH + gap);
      const rect = document.createElementNS(svgNS, 'rect');
      rect.setAttribute('x', x1);
      rect.setAttribute('y', y);
      rect.setAttribute('width', w);
      rect.setAttribute('height', rowH);
      rect.setAttribute('rx', '1');
      rect.setAttribute('fill', v.is_insecure ? 'var(--color-red-glow)' : 'var(--color-nix-500)');
      rect.setAttribute('opacity', v.is_insecure ? '0.75' : '0.85');
      svg.appendChild(rect);
      if (w > 30) {
        const label = document.createElementNS(svgNS, 'text');
        label.setAttribute('x', Math.min(980, x2 + 4));
        label.setAttribute('y', y + rowH - 1.5);
        label.setAttribute('font-family', 'JetBrains Mono');
        label.setAttribute('font-size', '8.5');
        label.setAttribute('fill', 'var(--color-fog-3)');
        label.textContent = v.version;
        svg.appendChild(label);
      }
    });

    el.appendChild(svg);
  }

  // ---------- stats / header populate ----------
  async function loadBoot() {
    const healthP = api(`${API_BASE}/health`)
      .then(({ json }) => {
        STATE.health = json;
      })
      .catch(() => {});
    const statsP = api(`${API_BASE}/stats`)
      .then(({ json }) => {
        STATE.stats = json.data || json;
      })
      .catch(() => {});
    await Promise.all([healthP, statsP]);

    await refreshMetrics();
    renderHeaderStrip();
    renderHeroStats();
    renderSelfhostCard();

    // keep metrics live — cheap in-memory query, no db work
    setInterval(refreshMetrics, 30_000);
  }

  async function refreshMetrics() {
    try {
      const { json } = await api(`${API_BASE}/metrics`);
      STATE.metrics = json;
      renderActivityCard(json);
      renderLatencyCard(json);
      refreshHeaderLatency(json);
    } catch {
      // leave previous values in place if a poll fails
    }
  }

  function renderHeaderStrip() {
    const stats = STATE.stats;
    const health = STATE.health;
    const strip = document.querySelector('header .h-6');
    if (!strip) return;

    const operational = !!health;
    const dot = operational
      ? '<span class="inline-block w-1.5 h-1.5 rounded-full" style="background: var(--color-green-glow); box-shadow: 0 0 6px oklch(0.74 0.15 150 / 0.6);"></span>'
      : '<span class="inline-block w-1.5 h-1.5 rounded-full" style="background: var(--color-red-glow); box-shadow: 0 0 6px oklch(0.66 0.19 25 / 0.6);"></span>';
    const opTxt = operational ? 'api operational' : 'api unreachable';

    const lastDate = stats?.last_indexed_date ? fmtDate(stats.last_indexed_date) : '—';
    const commit = health?.index_commit || stats?.last_indexed_commit || '';
    const oldest = stats?.oldest_commit_date ? new Date(stats.oldest_commit_date).toISOString().slice(0, 7) : '—';
    const newest = stats?.newest_commit_date ? new Date(stats.newest_commit_date).toISOString().slice(0, 7) : '—';

    strip.innerHTML = `
      <span id="headerStatus" class="flex items-center gap-1.5">
        ${dot}
        ${opTxt}
      </span>
      <span class="text-[var(--color-ink-4)]">│</span>
      <span>index · <span class="text-[var(--color-fog-2)]">${lastDate}</span>${commit ? ` · commit <span class="text-[var(--color-fog-2)]">${shortHash(commit)}</span>` : ''}</span>
      <span class="text-[var(--color-ink-4)]">│</span>
      <span>nixpkgs · <span class="text-[var(--color-fog-2)]">${oldest} → ${newest}</span></span>
      <span class="flex-1"></span>
      <span class="hidden md:inline">press <span class="kbd">/</span> to focus</span>`;

    // version tag in hero line — find by its original text
    const heroVer = [...document.querySelectorAll('main .mono')].find((el) =>
      el.textContent.includes('NIX VERSION INDEX'),
    );
    if (heroVer && health?.version) {
      heroVer.innerHTML = `// NIX VERSION INDEX &nbsp;·&nbsp; v${escapeHtml(health.version)}`;
    }
  }

  function refreshHeaderLatency(metrics) {
    const el = document.getElementById('headerStatus');
    const p50 = metrics?.latency?.p50_ms;
    if (!el || p50 == null) return;
    const dot = el.querySelector('span') || null;
    const dotHtml = dot ? dot.outerHTML : '';
    const operational = STATE.health ? 'api operational' : 'api unreachable';
    const latencyTxt = metrics.latency.samples > 0 ? ` · p50 ${p50.toFixed(1)}ms` : '';
    el.innerHTML = `${dotHtml} ${operational}${latencyTxt}`;
  }

  function renderHeroStats() {
    const s = STATE.stats;
    const aside = document.querySelector('aside.mono.select-none');
    if (!aside || !s) return;
    const oldest = s.oldest_commit_date ? new Date(s.oldest_commit_date) : null;
    const newest = s.newest_commit_date ? new Date(s.newest_commit_date) : null;
    const years = oldest && newest
      ? Math.max(1, Math.round((newest - oldest) / (365.25 * 24 * 3600e3)))
      : null;
    const histTxt = years != null ? `${years}+ yrs` : '—';
    aside.innerHTML = `
      <div class="ascii-rule text-[10px] leading-none mb-2">┌─ INDEX STATS ──────────────────────┐</div>
      <div class="grid grid-cols-2 gap-x-6 gap-y-1 px-3">
        <span>packages</span><span class="text-right text-[var(--color-fog-0)] tabular-nums">${fmtNum(s.unique_names)}</span>
        <span>versions</span><span class="text-right text-[var(--color-fog-0)] tabular-nums">${fmtNum(s.unique_versions)}</span>
        <span>records</span><span class="text-right text-[var(--color-fog-0)] tabular-nums">${fmtNum(s.total_ranges)}</span>
        <span>latest</span><span class="text-right text-[var(--color-fog-0)] tabular-nums mono">${s.last_indexed_commit ? '#' + shortHash(s.last_indexed_commit) : '—'}</span>
        <span>history</span><span class="text-right text-[var(--color-fog-0)] tabular-nums">${histTxt}</span>
      </div>
      <div class="ascii-rule text-[10px] leading-none mt-2">└────────────────────────────────────┘</div>`;
  }

  function renderLatencyCard(metrics) {
    const panel = document.querySelectorAll('#stats .panel')[1];
    if (!panel) return;
    // The big three-number readout is the first .tabular-nums inside the panel.
    const main = [...panel.querySelectorAll('.tabular-nums')][0];
    const lat = metrics?.latency;
    if (main) {
      if (!lat || lat.samples === 0) {
        main.innerHTML = `<span class="text-[var(--color-fog-4)] text-[18px]">waiting for samples…</span>`;
      } else {
        const fmt = (v) => (v < 10 ? v.toFixed(1) : Math.round(v));
        main.innerHTML = `
          ${fmt(lat.p50_ms)}<span class="text-[var(--color-fog-4)] text-[16px]">ms</span>
          <span class="mx-2 text-[var(--color-ink-4)] text-[16px]">/</span>
          ${fmt(lat.p95_ms)}<span class="text-[var(--color-fog-4)] text-[16px]">ms</span>
          <span class="mx-2 text-[var(--color-ink-4)] text-[16px]">/</span>
          ${fmt(lat.p99_ms)}<span class="text-[var(--color-fog-4)] text-[16px]">ms</span>`;
      }
    }
    const sub = panel.querySelector('.mt-5');
    if (sub) {
      const s = STATE.stats;
      const sampleTxt = lat?.samples > 0
        ? `from <span class="text-[var(--color-fog-0)]">${fmtNum(lat.samples)}</span> recent api requests`
        : `awaiting traffic`;
      sub.innerHTML = `
        records indexed · <span class="text-[var(--color-fog-0)]">${fmtNum(s?.total_ranges)}</span><br/>
        ${sampleTxt}`;
    }
  }

  function renderActivityCard(metrics) {
    const buckets = metrics?.activity || [];
    const counts = buckets.map((b) => b.count || 0);
    const max = Math.max(1, ...counts);
    const bars = counts
      .map((c) => {
        const h = c === 0 ? 3 : 6 + (c / max) * 48;
        const idle = c === 0;
        return `<span class="inline-block rounded-[1px] transition" style="width:6px; height:${h.toFixed(1)}px; background:${idle ? 'var(--color-ink-4)' : 'var(--color-green-glow)'}; opacity:${idle ? 0.5 : (0.55 + (h / 54) * 0.4).toFixed(2)};" title="${c} req${c === 1 ? '' : 's'}"></span>`;
      })
      .join('');
    cache('activityBars').innerHTML = bars;

    const summary = document.getElementById('activitySummary');
    if (summary) {
      const total = metrics?.runtime?.total_requests ?? 0;
      const uptime = fmtDuration(metrics?.runtime?.uptime_seconds ?? 0);
      summary.innerHTML = `uptime <span class="text-[var(--color-fog-0)]">${uptime}</span> · <span class="text-[var(--color-fog-0)]">${fmtNum(total)}</span> req${total === 1 ? '' : 's'}`;
    }
  }

  function fmtDuration(totalSeconds) {
    if (!Number.isFinite(totalSeconds) || totalSeconds < 0) return '—';
    const s = Math.floor(totalSeconds);
    if (s < 60) return `${s}s`;
    const m = Math.floor(s / 60);
    if (m < 60) return `${m}m ${s % 60}s`;
    const h = Math.floor(m / 60);
    if (h < 24) return `${h}h ${m % 60}m`;
    const d = Math.floor(h / 24);
    return `${d}d ${h % 24}h`;
  }

  function renderSelfhostCard() {
    const panel = document.querySelectorAll('#stats .panel')[2];
    if (!panel) return;
    const link = panel.querySelector('a.btn');
    if (link) {
      link.href = 'https://utensils.io/nxv/';
      link.target = '_blank';
      link.rel = 'noopener';
    }
  }

  // ---------- command palette ----------
  const PALETTE_ITEMS = [
    { cat: 'jump', label: 'python',     hint: 'popular package', action: () => runExample('python') },
    { cat: 'jump', label: 'nodejs',     hint: 'popular package', action: () => runExample('nodejs') },
    { cat: 'jump', label: 'ruby',       hint: 'popular package', action: () => runExample('ruby') },
    { cat: 'jump', label: 'gcc',        hint: 'toolchain',        action: () => runExample('gcc') },
    { cat: 'jump', label: 'postgresql', hint: 'databases',        action: () => runExample('postgresql') },
    { cat: 'jump', label: 'ffmpeg',     hint: 'media',            action: () => runExample('ffmpeg') },
    { cat: 'cmd',  label: 'toggle exact match',        hint: 'filter', action: () => { cycleFilter('exact'); renderFilterChips(); runSearch(); } },
    { cat: 'cmd',  label: 'include insecure packages', hint: 'filter', action: () => { cycleFilter('includeInsecure'); renderFilterChips(); runSearch(); } },
    { cat: 'cmd',  label: 'sort by name',  hint: 'sort', action: () => { STATE.filters.sort = 'name'; renderFilterChips(); runSearch(); } },
    { cat: 'cmd',  label: 'sort by date',  hint: 'sort', action: () => { STATE.filters.sort = 'date'; renderFilterChips(); runSearch(); } },
    { cat: 'cmd',  label: 'sort by version', hint: 'sort', action: () => { STATE.filters.sort = 'version'; renderFilterChips(); runSearch(); } },
    { cat: 'cmd',  label: 'clear filters', hint: 'reset', action: () => { STATE.filters = { exact: false, version: '', arch: '', license: '', sort: 'date', includeInsecure: false }; renderFilterChips(); runSearch(); } },
    { cat: 'go',   label: 'open api docs',   hint: '/docs',         action: () => { window.location.href = '/docs'; } },
    { cat: 'go',   label: 'openapi spec',    hint: '/openapi.json', action: () => { window.open('/openapi.json', '_blank'); } },
    { cat: 'go',   label: 'github',          hint: 'source',        action: () => window.open('https://github.com/utensils/nxv', '_blank') },
    { cat: 'go',   label: 'install guide',   hint: 'docs',          action: () => window.open('https://utensils.io/nxv/', '_blank') },
  ];
  let paletteIndex = 0;
  let paletteItems = [];

  function openPalette() {
    const p = cache('palette');
    p.classList.remove('hidden');
    p.classList.add('flex');
    const input = cache('paletteInput');
    input.value = '';
    input.focus();
    renderPalette('');
  }
  function closePalette() {
    const p = cache('palette');
    p.classList.add('hidden');
    p.classList.remove('flex');
  }
  function renderPalette(q) {
    q = q.trim().toLowerCase();
    paletteItems = PALETTE_ITEMS.filter((it) => !q || it.label.toLowerCase().includes(q) || it.cat.includes(q));
    // add ad-hoc "search for q" entry if user typed something
    if (q) {
      paletteItems.unshift({
        cat: 'run',
        label: `search: ${q}`,
        hint: '/api/v1/search',
        action: () => {
          STATE.query = q;
          cache('searchInput').value = q;
          runSearch();
        },
      });
    }
    paletteIndex = 0;
    cache('paletteList').innerHTML = paletteItems
      .map((it, i) => `
        <li data-idx="${i}" class="palette-item px-4 py-2 flex items-center gap-3 cursor-pointer mono text-[12.5px] ${i === 0 ? 'bg-[var(--color-ink-2)]' : ''}">
          <span class="chip" style="min-width: 36px; justify-content: center;">${escapeHtml(it.cat)}</span>
          <span class="text-[var(--color-fog-0)]">${escapeHtml(it.label)}</span>
          <span class="text-[var(--color-fog-4)]">${escapeHtml(it.hint)}</span>
          <span class="flex-1"></span>
          ${i === 0 ? '<span class="mono text-[10px] text-[var(--color-fog-4)]">↵ select</span>' : ''}
        </li>
      `)
      .join('');
    $$('#paletteList .palette-item').forEach((el, i) => {
      el.addEventListener('click', () => { paletteItems[i].action(); closePalette(); });
      el.addEventListener('mouseenter', () => { paletteIndex = i; highlightPalette(); });
    });
  }
  function highlightPalette() {
    $$('#paletteList .palette-item').forEach((el, i) => {
      el.classList.toggle('bg-[var(--color-ink-2)]', i === paletteIndex);
    });
  }

  // ---------- wiring ----------
  function runExample(q) {
    STATE.query = q;
    cache('searchInput').value = q;
    runSearch();
    window.scrollTo({ top: cache('resultsSection').offsetTop - 80, behavior: 'smooth' });
  }

  function wire() {
    let t;
    cache('searchInput').addEventListener('input', (e) => {
      clearTimeout(t);
      STATE.query = e.target.value;
      t = setTimeout(() => runSearch(), 220);
    });
    cache('searchInput').addEventListener('keydown', (e) => {
      if (e.key === 'Enter') { clearTimeout(t); runSearch(); }
    });

    $$('.chip[data-filter]').forEach((el) => {
      el.addEventListener('click', () => {
        const k = el.dataset.filter;
        cycleFilter(k === 'include-insecure' ? 'includeInsecure' : k);
        renderFilterChips();
        runSearch();
      });
    });

    $$('.chip.example').forEach((el) => {
      el.addEventListener('click', () => runExample(el.textContent.trim()));
    });

    $$('[data-view]').forEach((el) => {
      el.addEventListener('click', () => {
        $$('[data-view]').forEach((o) => {
          o.classList.remove('hairline');
          o.classList.remove('text-[var(--color-fog-0)]');
          o.classList.add('text-[var(--color-fog-4)]');
        });
        el.classList.add('hairline');
        el.classList.add('text-[var(--color-fog-0)]');
        el.classList.remove('text-[var(--color-fog-4)]');
        STATE.view = el.dataset.view;
        syncUrl();
      });
    });

    $$('[data-close]').forEach((el) => el.addEventListener('click', closeDrawer));

    cache('paletteTrigger').addEventListener('click', openPalette);
    $$('[data-palette-close]').forEach((el) => el.addEventListener('click', closePalette));
    cache('paletteInput').addEventListener('input', (e) => renderPalette(e.target.value));
    cache('paletteInput').addEventListener('keydown', (e) => {
      const items = $$('#paletteList .palette-item');
      if (e.key === 'ArrowDown') { e.preventDefault(); paletteIndex = Math.min(items.length - 1, paletteIndex + 1); highlightPalette(); }
      else if (e.key === 'ArrowUp') { e.preventDefault(); paletteIndex = Math.max(0, paletteIndex - 1); highlightPalette(); }
      else if (e.key === 'Enter') {
        e.preventDefault();
        if (items[paletteIndex]) items[paletteIndex].click();
      }
    });

    window.addEventListener('keydown', (e) => {
      const inField = ['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName);
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault(); openPalette();
      } else if (e.key === '/' && !inField) {
        e.preventDefault(); cache('searchInput').focus();
      } else if (e.key === 'Escape') {
        closePalette(); closeDrawer();
      }
    });
  }

  function applyViewToggleStyles() {
    $$('[data-view]').forEach((o) => {
      const active = o.dataset.view === STATE.view;
      o.classList.toggle('hairline', active);
      o.classList.toggle('text-[var(--color-fog-0)]', active);
      o.classList.toggle('text-[var(--color-fog-4)]', !active);
    });
  }

  function hasActiveState() {
    const f = STATE.filters;
    return !!(
      STATE.query ||
      f.version ||
      f.arch ||
      f.license ||
      (f.sort && f.sort !== 'date') ||
      f.exact ||
      f.includeInsecure ||
      STATE.page > 1
    );
  }

  // ---------- init ----------
  hydrateFromUrl();
  wire();
  cache('searchInput').value = STATE.query;
  renderFilterChips();
  applyViewToggleStyles();
  if (hasActiveState()) {
    runSearch({ resetPage: false });
  } else {
    renderWelcome();
  }
  loadBoot();
})();
