/* ============================================================
 * open4x-city.js — City management screen.
 *
 * Layout:
 *   ┌───────────┬──────────────────────────┬────────────┐
 *   │ cities    │ tiles (WebGL) · summary  │ production │
 *   │ list      │ districts                │ queue      │
 *   │           │ buildings · specialists  │            │
 *   └───────────┴──────────────────────────┴────────────┘
 *
 * Data sources (api adapter):
 *   /cities          → list + per-city detail
 *   /cities/:id/tiles → ownership / worked / center for the city hex view
 *   /world/snapshot   → already loaded; provides terrain backdrop
 * ============================================================ */
(function () {
  "use strict";

  const state = {
    cities: null,
    cityTiles: null,
    selectedId: null,
    cityHud: null,
  };

  // Public init — called from open4x.js once boot data is ready.
  async function init(data) {
    state.cities    = data.cities;
    state.cityTiles = data.cityTiles;
    state.selectedId = (state.cities.cities || [])[0]?.id;

    const screen = document.getElementById("screen-city");
    if (!screen) return;
    // replace stub with real layout
    screen.innerHTML = `
      <div class="city-screen">
        <aside class="city-side scrolly" id="city-list-side"></aside>
        <section class="city-main scrolly">
          <div class="city-hero">
            <div class="city-hero-l">
              <div class="kicker" id="city-hero-tag">CAPITAL</div>
              <h1 class="city-name" id="city-name">—</h1>
              <div class="city-sub" id="city-sub">—</div>
            </div>
            <div class="city-hero-stats" id="city-hero-stats"></div>
          </div>
          <div class="city-tiles" id="city-tiles">
            <canvas class="hex-canvas" id="city-hex-canvas"></canvas>
            <div class="hex-labels" id="city-hex-labels"></div>
          </div>
          <div class="city-cards">
            <div class="card city-card-yields" id="card-yields"></div>
            <div class="card city-card-growth" id="card-growth"></div>
            <div class="card city-card-population" id="card-pop"></div>
            <div class="card city-card-districts" id="card-districts"></div>
            <div class="card city-card-buildings" id="card-buildings"></div>
          </div>
        </section>
        <aside class="city-prod scrolly" id="city-prod-side"></aside>
      </div>
    `;
    // populate everything EXCEPT the WebGL hex view (which requires
    // the canvas to be visible & sized — handled on first repaint).
    renderList();
    const c = getCity(state.selectedId);
    if (c) {
      renderHero(c);
      renderCards(c);
      renderProduction(c);
    }
  }

  function getCity(id) {
    return (state.cities.cities || []).find(c => c.id === id);
  }
  function getCityTiles(id) {
    return (state.cityTiles?.cities || []).find(c => c.id === id);
  }

  // ── left rail: cities list ──────────────────────────────
  function renderList() {
    const root = document.getElementById("city-list-side");
    const items = state.cities.cities || [];
    root.innerHTML = `
      <div class="rail-h">
        <div class="title">Cities</div>
        <div class="meta">${items.length}</div>
      </div>
      <div class="rail-list">
        ${items.map(c => {
          const sumYield = (c.yields.food + c.yields.production + c.yields.gold + (c.yields.science||0) + (c.yields.culture||0));
          return `
            <button class="rail-item ${c.id === state.selectedId ? "active" : ""}" data-id="${c.id}">
              <div class="rail-item-l">
                <div class="rail-item-name">${c.name} ${c.capital ? "★" : ""}</div>
                <div class="rail-item-meta">pop ${c.population} · ${c.production?.current?.name || "—"} ${c.production?.current?.turns ? "· "+c.production.current.turns+"t" : ""}</div>
              </div>
              <div class="rail-item-r">
                <div class="num">${sumYield}</div>
                <div class="bar production" style="width:30px;"><i style="width:${Math.min(100, (c.production?.current?.progress / c.production?.current?.cost * 100) || 0)}%"></i></div>
              </div>
            </button>
          `;
        }).join("")}
      </div>
    `;
    root.querySelectorAll(".rail-item").forEach(el => {
      el.addEventListener("click", () => selectCity(el.dataset.id));
    });
  }

  // ── select city → re-render center + right rail ─────────
  function selectCity(id) {
    state.selectedId = id;
    document.querySelectorAll("#city-list-side .rail-item").forEach(el => {
      el.classList.toggle("active", el.dataset.id === id);
    });
    const c = getCity(id);
    if (!c) return;
    renderHero(c);
    renderTiles(c);
    renderCards(c);
    renderProduction(c);
  }

  // ── hero strip ──────────────────────────────────────────
  function renderHero(c) {
    document.getElementById("city-hero-tag").textContent = c.capital ? "Capital" : "City";
    document.getElementById("city-name").textContent = c.name;
    document.getElementById("city-sub").textContent =
      `${c.population} pop · ${c.position.q},${c.position.r} · loyalty ${Math.round(c.loyalty.current)}%`;
    const stats = document.getElementById("city-hero-stats");
    stats.innerHTML = [
      ["food",       "F", c.yields.food],
      ["production", "P", c.yields.production],
      ["gold",       "G", c.yields.gold],
      ["science",    "S", c.yields.science],
      ["culture",    "C", c.yields.culture],
      ["faith",      "F", c.yields.faith],
    ].map(([cls, g, v]) => `
      <div class="hero-stat ${cls}">
        <div class="num">${v}</div>
        <div class="kicker">${cls}</div>
      </div>
    `).join("");
  }

  // ── center map ──────────────────────────────────────────
  function renderTiles(c) {
    const tiles = getCityTiles(c.id);
    const status = new Map();
    if (tiles) {
      (tiles.tiles || []).forEach(t => {
        const wq = ((c.position.q + t.offset[0]) % window.WORLD.width + window.WORLD.width) % window.WORLD.width;
        const wr = c.position.r + t.offset[1];
        status.set(wq + "," + wr, t.status);
      });
    }
    const canvas = document.getElementById("city-hex-canvas");
    const field  = document.getElementById("city-tiles");
    const labels = document.getElementById("city-hex-labels");
    if (!state.cityHud) {
      state.cityHud = window.HexHud.create(canvas, field);
    }
    const cam = {
      x: c.position.q, y: c.position.r, zoom: 0.78,
      sel: { q: c.position.q, r: c.position.r },
    };
    state.cam = cam;
    state.opts = {
      centerHex: { q: c.position.q, r: c.position.r },
      viewRadius: 5,
      ownershipRadius: 3,
      tileStatus: status,
    };
    // First render — synchronously (canvas is already sized by the
    // time repaint() runs). Second render in next frame in case the
    // first one fired before WebGL was fully ready.
    state.cityHud.render(cam, state.opts);
    renderCityLabels(c, labels, field, cam, state.opts);
    requestAnimationFrame(() => {
      state.cityHud.render(cam, state.opts);
      renderCityLabels(c, labels, field, cam, state.opts);
    });

    // click → re-render selection
    if (!field._cityClickBound) {
      field._cityClickBound = true;
      field.addEventListener("click", (e) => {
        if (!state.cityHud) return;
        const hit = state.cityHud.pick(e.clientX, e.clientY, state.cam);
        if (!hit) return;
        // accept clicks inside the radius only
        const a = cubeFromOff(hit.q, hit.r);
        const b = cubeFromOff(state.opts.centerHex.q, state.opts.centerHex.r);
        const d = Math.max(Math.abs(a.x - b.x), Math.abs(a.y - b.y), Math.abs(a.z - b.z));
        if (d > state.opts.viewRadius) return;
        state.cam.sel = { q: hit.q, r: hit.r };
        state.cityHud.render(state.cam, state.opts);
        renderCityLabels(getCity(state.selectedId), labels, field, state.cam, state.opts);
      });
    }
  }

  function cubeFromOff(q, r) {
    const x = q - ((r - (r & 1)) >> 1);
    const z = r;
    return { x, y: -x - z, z };
  }

  function renderCityLabels(c, layer, field, cam, opts) {
    layer.innerHTML = "";
    const W = window.WORLD;
    if (!W) return;
    const HEX_R = 46;
    const SQRT3 = Math.sqrt(3);
    const cssW = field.clientWidth, cssH = field.clientHeight;
    const R = HEX_R * cam.zoom, hw = SQRT3 * R, hh = 1.5 * R;
    const camP = (cam.y | 0) & 1;
    const camPx = (cam.x + camP * 0.5) * hw;
    const camPy = cam.y * hh;
    const offX = cssW * 0.5 - camPx;
    const offY = cssH * 0.5 - camPy;

    function placeAt(q, r, html) {
      const p = (r & 1) ? 0.5 : 0;
      const cx = (q + p) * hw + offX;
      const cy = r * hh + offY;
      if (cx < -40 || cx > cssW + 40 || cy < -40 || cy > cssH + 40) return;
      const el = document.createElement("div");
      el.className = "hex-label";
      el.style.left = cx + "px";
      el.style.top  = cy + "px";
      el.innerHTML = html;
      layer.appendChild(el);
    }

    const center = opts.centerHex;
    const viewR  = opts.viewRadius;
    const status = opts.tileStatus;
    const cubeC = cubeFromOff(center.q, center.r);

    for (let dr = -viewR; dr <= viewR; dr++) {
      for (let dq = -viewR; dq <= viewR; dq++) {
        const q = center.q + dq;
        const r = center.r + dr;
        const a = cubeFromOff(q, r);
        const d = Math.max(Math.abs(a.x - cubeC.x), Math.abs(a.y - cubeC.y), Math.abs(a.z - cubeC.z));
        if (d > viewR) continue;
        const t = window.tileAt(q, r);
        if (t.oob || t.unloaded || t.fog) continue;
        const wq = t.wq;
        if (t.city) {
          placeAt(q, r + 0.6, `<div class="hl-city">${t.city.name} ${t.city.capital ? "★" : ""}</div>`);
          continue;
        }
        const st = status && status.get(wq + "," + r);
        if (t.resource && (st === "working" || st === "idle")) {
          placeAt(q, r + 0.55, `<div class="hl-res">${t.resource}</div>`);
        }
        if (cam.sel && wq === cam.sel.q && r === cam.sel.r) {
          const parts = [];
          parts.push(`<div class="hl-terrain">${t.terrain || "—"}</div>`);
          const ys = t.yields || {};
          const yStr = [ys.f?`${ys.f}F`:"", ys.p?`${ys.p}P`:"", ys.g?`${ys.g}G`:""].filter(Boolean).join(" ");
          if (yStr) parts.push(`<div class="hl-yields">${yStr}</div>`);
          parts.push(`<div class="hl-id">${wq},${r}</div>`);
          placeAt(q, r + 0.6, parts.join(""));
        }
      }
    }
  }

  // ── center cards ────────────────────────────────────────
  function renderCards(c) {
    // YIELDS
    document.getElementById("card-yields").innerHTML = `
      <div class="card-h"><span class="title">Yields</span><span class="meta">per turn</span></div>
      <div class="card-b">
        <div class="yields-grid">
          ${["food","production","gold","science","culture","faith"].map(k => `
            <div class="yield-row">
              <span class="dot ${k}"></span>
              <span class="label">${cap(k)}</span>
              <span class="num">${c.yields[k] || 0}</span>
            </div>
          `).join("")}
        </div>
      </div>
    `;

    // GROWTH
    const gr = c.growth || {};
    const grPct = gr.food_needed ? Math.round((gr.food_stored / gr.food_needed) * 100) : 0;
    document.getElementById("card-growth").innerHTML = `
      <div class="card-h"><span class="title">Growth</span><span class="meta">${gr.turns || "—"} turns</span></div>
      <div class="card-b">
        <div class="growth-line"><span class="mono">${gr.food_stored || 0}/${gr.food_needed || 0}</span> food stored</div>
        <div class="bar food" style="margin-top:6px;"><i style="width:${grPct}%"></i></div>
        <div class="hr" style="margin: 12px 0;"></div>
        <div class="growth-line"><span class="kicker">Housing</span> <span class="num">${c.housing.used} / ${c.housing.max}</span></div>
        <div class="growth-line" style="margin-top:6px;"><span class="kicker">Amenities</span> <span class="num ${c.amenities < 0 ? "neg" : c.amenities > 0 ? "pos" : ""}">${c.amenities >= 0 ? "+" : ""}${c.amenities}</span></div>
      </div>
    `;

    // POPULATION
    const total = c.population;
    document.getElementById("card-pop").innerHTML = `
      <div class="card-h"><span class="title">Population</span><span class="meta">${total} citizens</span></div>
      <div class="card-b">
        <div class="pop-strip">
          ${Array.from({length: total}, (_, i) => {
            const isSpec = i < (c.citizens_specialists || 0);
            const isWork = !isSpec && i < (c.citizens_specialists||0) + (c.citizens_working||0);
            const cls = isSpec ? "spec" : isWork ? "work" : "idle";
            return `<span class="pop-dot ${cls}" title="${cls}"></span>`;
          }).join("")}
        </div>
        <div class="hr" style="margin: 10px 0;"></div>
        <div class="growth-line"><span class="kicker">Focus</span> <span class="chip">${cap(c.citizen_focus || "default")}</span></div>
        ${c.specialists ? `
          <div class="spec-grid">
            ${Object.entries(c.specialists).map(([k, v]) => `
              <div class="spec-row"><span class="kicker">${cap(k)}</span><span class="num">${v}</span></div>
            `).join("")}
          </div>
        ` : ""}
      </div>
    `;

    // DISTRICTS
    document.getElementById("card-districts").innerHTML = `
      <div class="card-h"><span class="title">Districts</span><span class="meta">${(c.districts||[]).filter(d => d.status === "built").length} built</span></div>
      <div class="card-b" style="padding: 8px 4px;">
        ${(c.districts || []).map(d => `
          <div class="dist-row dist-${d.status}">
            <span class="dist-mark">${d.status === "built" ? "■" : d.status === "queued" ? "▥" : d.status === "available" ? "□" : "·"}</span>
            <span class="dist-name">${d.name}</span>
            <span class="dist-status">${d.status}${d.adj_bonus ? ` · +${d.adj_bonus}` : ""}</span>
          </div>
        `).join("")}
      </div>
    `;

    // BUILDINGS
    document.getElementById("card-buildings").innerHTML = `
      <div class="card-h"><span class="title">Buildings</span><span class="meta">${(c.buildings||[]).filter(b => b.status === "built").length} built</span></div>
      <div class="card-b" style="padding: 8px 4px;">
        ${(c.buildings || []).map(b => `
          <div class="bldg-row bldg-${b.status}">
            <span class="bldg-name">${b.name}</span>
            <span class="bldg-yields mono">${b.yields || ""}</span>
            <span class="bldg-status">${b.status === "built" ? "✓" : b.status === "queued" ? "Q" : b.status === "wonder" ? "★" : "·"}</span>
          </div>
        `).join("")}
      </div>
    `;
  }

  // ── right rail: production queue ────────────────────────
  function renderProduction(c) {
    const root = document.getElementById("city-prod-side");
    const p = c.production || {};
    const cur = p.current;
    root.innerHTML = `
      <div class="rail-h">
        <div class="title">Production</div>
        <div class="meta">${(p.queue || []).length + (cur ? 1 : 0)} items</div>
      </div>
      <div class="rail-body">
        ${cur ? `
          <div class="prod-current">
            <div class="kicker">Building</div>
            <div class="prod-name">${cur.name}</div>
            <div class="prod-meta">${cur.turns ? cur.turns + " turns" : ""} · ${cur.progress || 0} / ${cur.cost || "?"} P</div>
            <div class="bar production" style="margin-top:8px;">
              <i style="width:${Math.min(100, Math.round((cur.progress || 0) / (cur.cost || 1) * 100))}%"></i>
            </div>
            <div class="prod-actions">
              <button class="btn ghost">Buy with gold</button>
              <button class="btn ghost">Buy with faith</button>
            </div>
          </div>
        ` : `<div class="prod-empty">Nothing in production</div>`}

        <div class="hr" style="margin: 14px 0;"></div>
        <div class="kicker">Queue</div>
        <div class="prod-queue">
          ${(p.queue || []).map((q, i) => `
            <div class="prod-q-row">
              <span class="prod-q-num">${i + 1}</span>
              <span class="prod-q-name">${q.name}</span>
              <span class="prod-q-meta">${q.turns || "—"}t</span>
              <button class="btn ghost prod-q-rm" data-pos="${i}">×</button>
            </div>
          `).join("") || `<div class="prod-empty">No items queued</div>`}
        </div>

        <div class="hr" style="margin: 14px 0;"></div>
        <div class="kicker">Add to queue</div>
        <div class="prod-suggest">
          ${(p.suggestions || []).map(s => `
            <button class="prod-sug-row">
              <span class="prod-sug-name">${s.name}</span>
              <span class="prod-sug-meta">${s.kind || ""} · ${s.turns || "—"}t</span>
            </button>
          `).join("") || `<div class="prod-empty">No suggestions</div>`}
        </div>
      </div>
    `;
  }

  function cap(s) { return s ? s[0].toUpperCase() + s.slice(1) : s; }

  // Repaint the WebGL hex map; called when the city screen becomes
  // visible (canvas pixel buffers go blank when display:none).
  // Also performs the FIRST render of the hex view — we defer it
  // here because WebGL on a hidden canvas (0×0) is unreliable.
  function repaint() {
    if (!state.selectedId) return;
    const c = getCity(state.selectedId);
    if (!c) return;
    // ensure the cards reflect current selection too
    renderHero(c);
    renderCards(c);
    renderProduction(c);
    renderTiles(c);
  }

  window.Open4XCity = { init, repaint };
})();
