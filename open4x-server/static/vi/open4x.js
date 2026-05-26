/* ============================================================
 * open4x.js — main wireup for the hi-fi 4X UI.
 *
 * Responsibilities:
 *  - Boot: fetch the slices needed for the initial render
 *  - Topbar: turn / era / resources
 *  - Tabs: screen switching with keyboard shortcuts (1-9)
 *  - HUD: WebGL hex map (via hex-webgl-hud.js), notifications,
 *         turn queue, minimap, context panel
 *
 * Backend swap:
 *  - api.config.mode = "json"  → uses local *.json fixtures
 *  - api.config.mode = "live"  → POSTs go to /api/v1/* with token
 * ============================================================ */
(function () {
  "use strict";

  // ── state ──────────────────────────────────────────────
  const state = {
    player: null,
    world:  null,
    cities: null,
    cityById: {},
    notifications: null,
    turnQueue: null,
    units: null,
    tech: null,
    civics: null,
    // HUD camera (axial, q & r in tile units)
    cam: { x: 14, y: 7, zoom: 1, sel: null },
    activeScreen: "hud",
    hud: null,            // HexHud instance for the main map
    miniHud: null,        // HexHud instance for the minimap
  };

  // ── world-snapshot adapters ────────────────────────────
  // The wireframe's hex renderer reads window.WORLD + window.tileAt.
  // We provide those by adapting the JSON-shaped snapshot.
  function buildWorldAccessors(snap) {
    const W  = snap.world;
    const ts = snap.tiles || [];
    const byKey = new Map();
    for (const t of ts) byKey.set(t.q + "," + t.r, t);
    window.WORLD  = W;
    window.tileAt = function (q, r) {
      // out-of-bounds in r
      if (!W.wrapY && (r < 0 || r >= W.height)) {
        return { oob: true, q, r, wq: ((q % W.width) + W.width) % W.width };
      }
      const wq = ((q % W.width) + W.width) % W.width;
      const k  = wq + "," + r;
      if (byKey.has(k)) {
        const t = byKey.get(k);
        return Object.assign({ q, wq, r }, t);
      }
      return { unloaded: true, q, wq, r };
    };
  }

  // ── BOOT ───────────────────────────────────────────────
  async function boot() {
    // pull the slices the HUD needs
    const [player, world, cities, units, notif, tq, tech, civics] = await Promise.all([
      api.playerState(),
      api.worldSnapshot(),
      api.cities(),
      api.units(),
      api.notifications(),
      api.turnQueue(),
      api.tech(),
      api.civics(),
    ]);
    state.player = player;
    state.world  = world;
    state.cities = cities;
    (cities.cities || []).forEach(c => { state.cityById[c.id] = c; });
    state.units = units;
    state.notifications = notif;
    state.turnQueue = tq;
    state.tech = tech;
    state.civics = civics;
    // city tiles need cities resolved first (live mode aggregates per id)
    state.cityTilesAll = await _fetchCityTiles(cities);

    buildWorldAccessors(world);
    bootTopbar();
    bootTabs();
    bootHud();
    bootNotifications();
    bootTurnQueue();
    bootEndTurn();
    bootMinimap();
    bootKeyboard();
    bootScreenModules();

    // initial map paint after layout settles
    requestAnimationFrame(() => {
      const sel = world.tiles && world.tiles.find(t => t.city) || world.tiles[0];
      if (sel) {
        state.cam.x = sel.q;
        state.cam.y = sel.r;
        state.cam.sel = { q: sel.q, r: sel.r };
      }
      renderHud();
      renderMinimap();
    });
  }

  // city-tiles.json is one fetch shared by the HUD's tooltip + city screen.
  // In json mode the bundled fixture has every city in one file; in live
  // mode the server only exposes /cities/:id/tiles, so we aggregate per id.
  async function _fetchCityTiles(cities) {
    if (api.config.mode !== "live") {
      const r = await fetch("./city-tiles.json", { cache: "no-cache" });
      if (!r.ok) return { cities: [] };
      return r.json();
    }
    const list = (cities && cities.cities) || [];
    const tiles = await Promise.all(
      list.map(c => api.cityTiles(c.id).catch(() => null))
    );
    return { cities: tiles.filter(Boolean) };
  }

  // hand the boot data to per-screen modules
  function bootScreenModules() {
    if (window.Open4XCity) {
      window.Open4XCity.init({
        cities: state.cities,
        cityTiles: state.cityTilesAll,
      });
    }
    if (window.Open4XTech) {
      window.Open4XTech.init();
    }
    if (window.Open4XCivics) window.Open4XCivics.init();
    if (window.Open4XUnits)  window.Open4XUnits.init();
    if (window.Open4XDipl)   window.Open4XDipl.init();
    if (window.Open4XGovt)   window.Open4XGovt.init();
    if (window.Open4XOverview) window.Open4XOverview.init();
    if (window.Open4XVictory)  window.Open4XVictory.init();
  }

  // ── TOPBAR ─────────────────────────────────────────────
  function bootTopbar() {
    const p = state.player;

    // game title — pull civ from world snapshot
    const civEl = document.getElementById("brand-civ");
    if (state.world && state.world.world && state.world.world.civ_name) {
      civEl.textContent = `${state.world.world.civ_name} · ${state.world.world.leader_name || ""}`;
    }

    document.getElementById("turn-val").textContent = p.turn;
    document.getElementById("turn-max").textContent = p.turn_max;
    document.getElementById("era-val").textContent  = p.era;
    document.getElementById("era-bar-fill").style.width = Math.round((p.era_progress || 0) * 100) + "%";

    // resources — gold, science, culture, faith, food, production, happiness
    const order = [
      { key: "gold",        cls: "gold",       glyph: "G", showValue: true,  tt: "Treasury" },
      { key: "science",     cls: "science",    glyph: "S", showValue: false, tt: "Science / turn" },
      { key: "culture",     cls: "culture",    glyph: "C", showValue: false, tt: "Culture / turn" },
      { key: "faith",       cls: "faith",      glyph: "F", showValue: false, tt: "Faith / turn" },
      { key: "food",        cls: "food",       glyph: "f", showValue: false, tt: "Food / turn (net empire)" },
      { key: "production",  cls: "production", glyph: "P", showValue: false, tt: "Production / turn" },
    ];
    const root = document.getElementById("resources");
    root.innerHTML = "";
    for (const r of order) {
      const data = p.resources[r.key];
      if (!data) continue;
      const el = document.createElement("div");
      el.className = `res ${r.cls}`;
      el.title = r.tt;
      const delta = data.per_turn;
      const deltaCls = delta > 0 ? "pos" : delta < 0 ? "neg" : "";
      const sign = delta > 0 ? "+" : "";
      if (r.showValue && data.value != null) {
        el.innerHTML = `
          <span class="res-glyph">${r.glyph}</span>
          <span class="res-val">${data.value.toLocaleString()}</span>
          <span class="res-delta ${deltaCls}">${sign}${delta}</span>
        `;
      } else {
        el.innerHTML = `
          <span class="res-glyph">${r.glyph}</span>
          <span class="res-delta ${deltaCls}" style="color: var(--ink);">${sign}${delta}</span>
        `;
      }
      root.appendChild(el);
    }

    // happiness pinned at the end
    const h = document.createElement("div");
    h.className = "res happy";
    h.title = "Happiness";
    const happy = p.happiness || 0;
    const hCls = happy > 0 ? "pos" : happy < 0 ? "neg" : "";
    h.innerHTML = `
      <span class="res-glyph">${happy >= 0 ? ":)" : ":("}</span>
      <span class="res-val ${hCls}">${happy >= 0 ? "+" : ""}${happy}</span>
    `;
    root.appendChild(h);
  }

  // ── TABS ───────────────────────────────────────────────
  function bootTabs() {
    document.querySelectorAll(".tab").forEach(tab => {
      tab.addEventListener("click", () => switchScreen(tab.dataset.screen));
    });
    // counts
    const cityCount = (state.cities.cities || []).length;
    const unitCount = (state.units.units || []).length;
    document.getElementById("tab-city-count").textContent = cityCount;
    document.getElementById("tab-unit-count").textContent = unitCount;
    // tech / civics badges: just an arrow (full name shown on hover via title)
    const cur = (state.tech.techs || []).find(t => t.status === "current");
    if (cur) {
      const b = document.getElementById("tab-tech-cur");
      const pct = Math.round((cur.progress || 0) / (cur.cost || 1) * 100);
      b.textContent = `${pct}%`;
      b.title = `Researching ${cur.name} (${cur.progress}/${cur.cost})`;
    }
    const curC = (state.civics.civics || []).find(c => c.status === "current");
    if (curC) {
      const b = document.getElementById("tab-civics-cur");
      const pct = Math.round((curC.progress || 0) / (curC.cost || 1) * 100);
      b.textContent = `${pct}%`;
      b.title = `Researching ${curC.name} (${curC.progress}/${curC.cost})`;
    } else {
      // hide if no current civic
      const b = document.getElementById("tab-civics-cur");
      b.style.display = "none";
    }
  }

  function switchScreen(id) {
    state.activeScreen = id;
    document.querySelectorAll(".screen").forEach(s => s.classList.toggle("active", s.dataset.screen === id));
    document.querySelectorAll(".tab").forEach(t => t.classList.toggle("active", t.dataset.screen === id));
    // canvases lose their pixel buffer when their container hides;
    // repaint after layout settles (rAF + a short timeout — rAF alone
    // can fire before the browser has finished sizing the freshly-
    // unhidden canvas, which leaves WebGL drawing into a stale 0×0
    // buffer until the next user interaction)
    const repaintScreen = () => {
      if (id === "hud")  { renderHud(); renderMinimap(); }
      if (id === "city" && window.Open4XCity && window.Open4XCity.repaint) window.Open4XCity.repaint();
    };
    requestAnimationFrame(repaintScreen);
    setTimeout(repaintScreen, 60);
  }

  // ── HUD MAP ────────────────────────────────────────────
  function bootHud() {
    const field  = document.getElementById("hud-map");
    const canvas = document.getElementById("hud-canvas");
    if (!window.HexHud) { console.warn("HexHud missing"); return; }
    state.hud = window.HexHud.create(canvas, field);

    // pan
    let drag = null;
    field.addEventListener("pointerdown", (e) => {
      if (e.button !== 0) return;
      field.setPointerCapture(e.pointerId);
      drag = { x: e.clientX, y: e.clientY, camX: state.cam.x, camY: state.cam.y, moved: false };
      field.classList.add("dragging");
    });
    field.addEventListener("pointermove", (e) => {
      if (!drag) return;
      const dx = e.clientX - drag.x;
      const dy = e.clientY - drag.y;
      if (Math.abs(dx) + Math.abs(dy) > 4) drag.moved = true;
      const R  = window.HexHud.HEX_R * state.cam.zoom;
      const hw = Math.sqrt(3) * R, hh = 1.5 * R;
      state.cam.x = drag.camX - dx / hw;
      state.cam.y = Math.max(-3, Math.min(state.world.world.height + 2, drag.camY - dy / hh));
      renderHud();
      renderMinimap();
    });
    const endDrag = () => { if (!drag) return; setTimeout(() => { drag = null; field.classList.remove("dragging"); }, 0); };
    field.addEventListener("pointerup", endDrag);
    field.addEventListener("pointercancel", endDrag);
    field.addEventListener("pointerleave", endDrag);

    // zoom
    field.addEventListener("wheel", (e) => {
      e.preventDefault();
      const delta = -Math.sign(e.deltaY) * 0.12;
      state.cam.zoom = Math.max(0.4, Math.min(1.8, state.cam.zoom + delta));
      renderHud();
    }, { passive: false });

    document.getElementById("zoom-in").addEventListener("click",  () => { state.cam.zoom = Math.min(1.8, state.cam.zoom + 0.18); renderHud(); });
    document.getElementById("zoom-out").addEventListener("click", () => { state.cam.zoom = Math.max(0.4, state.cam.zoom - 0.18); renderHud(); });
    document.getElementById("zoom-home").addEventListener("click", () => {
      const cap = (state.cities.cities || []).find(c => c.capital) || state.cities.cities[0];
      if (cap) { state.cam.x = cap.position.q; state.cam.y = cap.position.r; state.cam.zoom = 1; state.cam.sel = { q: cap.position.q, r: cap.position.r }; }
      renderHud(); renderMinimap();
    });

    // click → select tile
    field.addEventListener("click", (e) => {
      if (drag && drag.moved) return;
      if (!state.hud) return;
      const hit = state.hud.pick(e.clientX, e.clientY, state.cam);
      if (!hit) return;
      state.cam.sel = { q: hit.q, r: hit.r };
      renderHud();
      renderContext();
    });
  }

  function renderHud() {
    if (!state.hud) return;
    state.hud.render(state.cam);
    renderHexLabels();
    renderMinimapViewport();
  }

  function renderHexLabels() {
    const layer = document.getElementById("hud-labels");
    const field = document.getElementById("hud-map");
    layer.innerHTML = "";
    const cam = state.cam;
    const W = state.world.world;
    const HEX_R = window.HexHud.HEX_R;
    const SQRT3 = Math.sqrt(3);
    const cssW = field.clientWidth, cssH = field.clientHeight;
    const R = HEX_R * cam.zoom, hw = SQRT3 * R, hh = 1.5 * R;
    const tx = Math.ceil(cssW / hw) + 4, ty = Math.ceil(cssH / hh) + 4;
    const q0 = Math.floor(cam.x - tx/2), q1 = Math.ceil(cam.x + tx/2);
    const r0 = Math.floor(cam.y - ty/2), r1 = Math.ceil(cam.y + ty/2);
    const camP = (cam.y | 0) & 1;
    const camPx = (cam.x + camP * 0.5) * hw;
    const camPy = cam.y * hh;
    const offX = cssW * 0.5 - camPx, offY = cssH * 0.5 - camPy;

    function placeAt(q, r, html) {
      const p = (r & 1) ? 0.5 : 0;
      const cx = (q + p) * hw + offX;
      const cy = r * hh + offY;
      if (cx < -60 || cx > cssW + 60 || cy < -40 || cy > cssH + 40) return;
      const el = document.createElement("div");
      el.className = "hex-label";
      el.style.left = cx + "px"; el.style.top = cy + "px";
      el.innerHTML = html;
      layer.appendChild(el);
    }

    for (let r = r0; r <= r1; r++) {
      for (let q = q0; q <= q1; q++) {
        const t = window.tileAt(q, r);
        if (t.oob || t.unloaded || t.fog) continue;
        if (t.city && cam.zoom >= 0.6) {
          const cap = t.city.capital ? " ★" : "";
          placeAt(q, r + 0.62, `<div class="hl-city">${t.city.name}${cap}</div>`);
        }
        if (t.wonder && cam.zoom >= 0.85) {
          placeAt(q, r - 0.55, `<div class="hl-res">★ ${t.wonder.name}</div>`);
        }
        if (cam.sel && t.wq === cam.sel.q && r === cam.sel.r) {
          const parts = [];
          if (!t.city) parts.push(`<div class="hl-terrain">${t.terrain || "—"}</div>`);
          if (t.resource) parts.push(`<div class="hl-res">${t.resource}</div>`);
          parts.push(`<div class="hl-id">${t.wq},${r}</div>`);
          if (parts.length) placeAt(q, r + (t.city ? 0.95 : 0.55), parts.join(""));
        }
      }
    }
  }

  // ── CONTEXT PANEL ──────────────────────────────────────
  function renderContext() {
    const panel = document.getElementById("context-panel");
    const sel = state.cam.sel;
    if (!sel) {
      panel.innerHTML = `<div class="context-empty">Click a tile to inspect · drag to pan · scroll to zoom</div>`;
      return;
    }
    const t = window.tileAt(sel.q, sel.r);
    if (t.oob || t.unloaded) {
      panel.innerHTML = `<div class="context-empty">${t.oob ? "Out of bounds" : "Undiscovered"} · ${sel.q},${sel.r}</div>`;
      return;
    }

    const ys = t.yields || {};
    const yieldEls = [];
    if (ys.f) yieldEls.push(`<span class="y y-f">${ys.f} F</span>`);
    if (ys.p) yieldEls.push(`<span class="y y-p">${ys.p} P</span>`);
    if (ys.g) yieldEls.push(`<span class="y y-g">${ys.g} G</span>`);
    if (ys.sci) yieldEls.push(`<span class="y y-s">${ys.sci} S</span>`);
    if (ys.cul) yieldEls.push(`<span class="y y-c">${ys.cul} C</span>`);
    const yieldHtml = yieldEls.length ? `<span class="yields">${yieldEls.join("")}</span>` : `<span class="ctx-meta">no yields</span>`;

    // section: TILE
    let sections = "";
    sections += `
      <div class="ctx-section" style="min-width:200px;">
        <div class="ctx-title">Tile</div>
        <div class="ctx-h">${t.terrain || "Unknown"} <span class="coord">${t.wq},${sel.r}</span></div>
        <div class="ctx-meta">${yieldHtml}</div>
        ${t.resource ? `<div class="ctx-meta" style="margin-top:4px;"><span class="chip accent">${t.resource}</span></div>` : ""}
      </div>
    `;

    if (t.city) {
      sections += `
        <div class="ctx-section" style="min-width:200px;">
          <div class="ctx-title">City</div>
          <div class="ctx-h">${t.city.name} ${t.city.capital ? "★" : ""} <span class="coord">pop ${t.city.pop || "?"}</span></div>
          <div class="ctx-meta">${t.city.civ || "—"}</div>
        </div>
      `;
    }
    if (t.unit) {
      sections += `
        <div class="ctx-section" style="min-width:200px;">
          <div class="ctx-title">Unit</div>
          <div class="ctx-h">${t.unit.kind}${t.unit.name ? " · " + t.unit.name : ""}</div>
          <div class="ctx-meta">HP ${t.unit.hp || "?"} · MP ${t.unit.mp || "?"}</div>
        </div>
      `;
    }
    if (t.wonder) {
      sections += `
        <div class="ctx-section" style="min-width:200px;">
          <div class="ctx-title">Wonder</div>
          <div class="ctx-h">${t.wonder.name}</div>
          <div class="ctx-meta">${t.wonder.desc || ""}</div>
        </div>
      `;
    }

    // section: ACTIONS
    sections += `
      <div class="ctx-actions">
        ${t.city ? `<button class="btn" data-act="open-city">Open city</button>` : ""}
        ${t.unit ? `<button class="btn" data-act="open-unit">Open unit</button>` : ""}
        <button class="btn ghost" data-act="recenter">Recenter</button>
        <button class="btn ghost" data-act="close">Close</button>
      </div>
    `;

    panel.innerHTML = sections;
    panel.querySelectorAll("[data-act]").forEach(b => {
      b.addEventListener("click", () => {
        const a = b.dataset.act;
        if (a === "close") { state.cam.sel = null; renderHud(); renderContext(); }
        else if (a === "recenter") { state.cam.x = sel.q; state.cam.y = sel.r; renderHud(); renderMinimap(); }
        else if (a === "open-city") switchScreen("city");
        else if (a === "open-unit") switchScreen("units");
      });
    });
  }

  // ── NOTIFICATIONS ──────────────────────────────────────
  function bootNotifications() {
    document.getElementById("notif-turn").textContent = `T ${state.notifications.turn}`;
    document.getElementById("notif-clear").addEventListener("click", async () => {
      await api.dismissAllNotifications();
      state.notifications.notifications = [];
      renderNotifications();
    });
    renderNotifications();
  }

  function renderNotifications() {
    const list = document.getElementById("notif-list");
    const items = state.notifications.notifications || [];
    if (!items.length) {
      list.innerHTML = `<div class="empty-hint">All clear · no pending notifications</div>`;
      return;
    }
    list.innerHTML = items.map(n => {
      const kind = n.kind || "";
      const glyph = ({
        research: "🔬", environment: "≈", military: "⚔", builder: "✓",
        diplomacy: "◊", civic: "▦", city: "■", production: "▥",
        religion: "✦", economy: "$", era: "Δ"
      })[n.category] || "•";
      return `
        <div class="notif ${kind}" data-id="${n.id}" data-target='${JSON.stringify(n.target || null)}'>
          <span class="notif-icon">${glyph}</span>
          <div>
            <div class="notif-title">${escapeHtml(n.title)} <span class="notif-cat">${n.category || ""}</span></div>
            <div class="notif-desc">${escapeHtml(n.desc)}</div>
          </div>
        </div>
      `;
    }).join("");

    list.querySelectorAll(".notif").forEach(el => {
      el.addEventListener("click", () => {
        const target = JSON.parse(el.dataset.target || "null");
        if (!target) return;
        if (target.screen && target.screen !== "hud") switchScreen(target.screen);
        else if (target.q != null && target.r != null) {
          state.cam.x = target.q; state.cam.y = target.r; state.cam.sel = { q: target.q, r: target.r };
          renderHud(); renderContext(); renderMinimap();
        }
      });
    });
  }

  // ── TURN QUEUE ─────────────────────────────────────────
  function bootTurnQueue() {
    renderTurnQueue();
  }

  function renderTurnQueue() {
    const list = document.getElementById("tq-list");
    const items = state.turnQueue.items || [];
    const required = items.filter(i => i.required).length;
    const total = items.length;
    document.getElementById("tq-counter").textContent = `${total} pending` + (required ? ` · ${required} required` : "");

    if (!items.length) {
      list.innerHTML = `<div class="empty-hint">Queue empty · end turn ready</div>`;
    } else {
      list.innerHTML = items.map((item, idx) => `
        <div class="tq-item kind-${item.kind} ${item.required ? "required" : ""}" data-id="${item.id}" data-target='${JSON.stringify(item.target || null)}'>
          <span class="tq-num">${idx + 1}</span>
          <div>
            <div class="tq-title">${escapeHtml(item.title)} ${item.required ? '<span class="req-badge">required</span>' : ""}</div>
            <div class="tq-desc">${escapeHtml(item.desc)}</div>
          </div>
          ${item.skipLabel ? `<button class="tq-skip" data-skip="${item.id}">${escapeHtml(item.skipLabel)}</button>` : ""}
        </div>
      `).join("");

      list.querySelectorAll(".tq-item").forEach(el => {
        el.addEventListener("click", (e) => {
          if (e.target.matches("[data-skip]")) return;
          const target = JSON.parse(el.dataset.target || "null");
          if (!target) return;
          if (target.screen && target.screen !== "hud") switchScreen(target.screen);
          else if (target.q != null && target.r != null) {
            state.cam.x = target.q; state.cam.y = target.r; state.cam.sel = { q: target.q, r: target.r };
            renderHud(); renderContext(); renderMinimap();
          }
        });
      });
      list.querySelectorAll("[data-skip]").forEach(b => {
        b.addEventListener("click", async (e) => {
          e.stopPropagation();
          const id = b.dataset.skip;
          await api.skipQueueItem(id);
          state.turnQueue.items = state.turnQueue.items.filter(i => i.id !== id);
          renderTurnQueue();
          updateEndTurnState();
        });
      });
    }
    updateEndTurnState();
  }

  // ── END-TURN ───────────────────────────────────────────
  function bootEndTurn() {
    const btn = document.getElementById("end-turn-btn");
    btn.addEventListener("click", async () => {
      const required = (state.turnQueue.items || []).filter(i => i.required);
      if (required.length) {
        // Jump to the first required item's target
        const target = required[0].target;
        if (target && target.screen && target.screen !== "hud") switchScreen(target.screen);
        return;
      }
      btn.classList.add("disabled");
      btn.querySelector("span").textContent = "Resolving…";
      await api.endTurn();
      // in real life we'd re-fetch and re-render. for the mock, bump the turn.
      state.player.turn += 1;
      document.getElementById("turn-val").textContent = state.player.turn;
      btn.classList.remove("disabled");
      btn.querySelector("span").textContent = "End Turn";
    });
    updateEndTurnState();
  }

  function updateEndTurnState() {
    const items = state.turnQueue.items || [];
    const required = items.filter(i => i.required).length;
    const remain = items.length;
    const meta = document.getElementById("end-turn-meta");
    const summary = document.getElementById("queue-summary");
    const dot = meta.querySelector(".dot");
    dot.classList.toggle("ok", required === 0);
    if (!remain) {
      summary.textContent = "ready to end turn";
    } else if (required) {
      summary.textContent = `${required} required action${required === 1 ? "" : "s"}`;
    } else {
      summary.textContent = `${remain} optional · ready to end`;
    }
    const btn = document.getElementById("end-turn-btn");
    btn.classList.toggle("disabled", required > 0);
    btn.querySelector("span").textContent = required ? "Resolve required" : "End Turn";
  }

  // ── MINIMAP ────────────────────────────────────────────
  function bootMinimap() {
    const overlay = document.getElementById("minimap-overlay");
    let dragMM = false;
    overlay.addEventListener("pointerdown", (e) => { dragMM = true; jumpMinimap(e); });
    overlay.addEventListener("pointermove", (e) => { if (dragMM) jumpMinimap(e); });
    window.addEventListener("pointerup",   () => { dragMM = false; });
  }
  function jumpMinimap(e) {
    const overlay = document.getElementById("minimap-overlay");
    const rect = overlay.getBoundingClientRect();
    const fx = (e.clientX - rect.left) / rect.width;
    const fy = (e.clientY - rect.top)  / rect.height;
    state.cam.x = fx * state.world.world.width;
    state.cam.y = fy * state.world.world.height;
    renderHud();
    renderMinimap();
  }

  function renderMinimap() {
    const canvas = document.getElementById("minimap-canvas");
    const ctx = canvas.getContext("2d");
    const W = state.world.world;
    const ts = state.world.tiles || [];
    const cw = canvas.width, ch = canvas.height;
    ctx.fillStyle = "#efece4";
    ctx.fillRect(0, 0, cw, ch);
    // simple coloring per terrain → muted palette
    const PAL = {
      Ocean: "#b5c2d3", Coast: "#c9d3df",
      Plains: "#e3dabf", Grass: "#cbd7a6",
      Hills: "#cfbf9a", Forest: "#a9bf8c",
      Desert: "#e8d8a8", Tundra: "#dde0d8",
      Mountain: "#a39c93", Mtn: "#a39c93",
      Floodpl: "#dac99e", Floodplain: "#dac99e",
      Jungle: "#a7b58d", Marsh: "#bcc0a3",
      River: "#bcccdc", Snow: "#eef0ec", Reef: "#c4d3d2",
    };
    const tw = cw / W.width;
    const th = ch / W.height;
    for (const t of ts) {
      ctx.fillStyle = PAL[t.terrain] || "#e0dccf";
      ctx.fillRect(Math.floor(t.q * tw), Math.floor(t.r * th), Math.ceil(tw) + 1, Math.ceil(th) + 1);
      if (t.city) {
        ctx.fillStyle = t.city.capital ? "#1a1a1a" : "#3a3a37";
        const sz = 3;
        ctx.fillRect(Math.floor(t.q * tw) - 1, Math.floor(t.r * th) - 1, sz, sz);
      } else if (t.owner) {
        ctx.fillStyle = "rgba(31,107,107,.18)";
        ctx.fillRect(Math.floor(t.q * tw), Math.floor(t.r * th), Math.ceil(tw) + 1, Math.ceil(th) + 1);
      } else if (t.unit) {
        ctx.fillStyle = "#2d7a4a";
        ctx.fillRect(Math.floor(t.q * tw), Math.floor(t.r * th), Math.ceil(tw) + 1, Math.ceil(th) + 1);
      }
    }
    renderMinimapViewport();
  }
  function renderMinimapViewport() {
    const vp = document.getElementById("minimap-vp");
    const overlay = document.getElementById("minimap-overlay");
    if (!state.world) return;
    const W = state.world.world;
    const HEX_R = window.HexHud ? window.HexHud.HEX_R : 46;
    const R = HEX_R * state.cam.zoom;
    const hw = Math.sqrt(3) * R, hh = 1.5 * R;
    const field = document.getElementById("hud-map");
    const fw = field.clientWidth, fh = field.clientHeight;
    const tilesX = fw / hw, tilesY = fh / hh;
    const rectW = (tilesX / W.width) * overlay.clientWidth;
    const rectH = (tilesY / W.height) * overlay.clientHeight;
    const cx = (state.cam.x / W.width) * overlay.clientWidth;
    const cy = (state.cam.y / W.height) * overlay.clientHeight;
    vp.style.width = rectW + "px";
    vp.style.height = rectH + "px";
    vp.style.left = (cx - rectW / 2) + "px";
    vp.style.top  = (cy - rectH / 2) + "px";
  }

  // ── KEYBOARD ───────────────────────────────────────────
  function bootKeyboard() {
    document.addEventListener("keydown", (e) => {
      // tab numbers 1-9
      const screens = ["hud","city","units","tech","civics","dipl","govt","overview","victory"];
      if (e.key >= "1" && e.key <= "9" && !e.metaKey && !e.ctrlKey && !e.altKey) {
        const idx = parseInt(e.key, 10) - 1;
        if (screens[idx]) {
          switchScreen(screens[idx]);
          e.preventDefault();
        }
      }
      // enter → end turn
      if (e.key === "Enter" && state.activeScreen === "hud") {
        document.getElementById("end-turn-btn").click();
        e.preventDefault();
      }
      // escape → clear selection
      if (e.key === "Escape") {
        state.cam.sel = null;
        renderHud();
        renderContext();
      }
    });
    // resize handler
    let rt;
    window.addEventListener("resize", () => {
      clearTimeout(rt);
      rt = setTimeout(() => { renderHud(); renderMinimap(); }, 80);
    });
  }

  // ── utils ──────────────────────────────────────────────
  function escapeHtml(s) {
    if (s == null) return "";
    return String(s)
      .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;").replace(/'/g, "&#39;");
  }

  // ── go ──────────────────────────────────────────────
  function _bootSafely() {
    boot().catch(err => {
      console.error("boot failed:", err);
      document.body.insertAdjacentHTML("beforeend",
        `<div style="position:fixed;inset:0;display:flex;align-items:center;justify-content:center;background:rgba(247,246,243,.95);font-family:var(--font-sans);color:var(--bad)">
          <div style="text-align:center;max-width:500px;">
            <div style="font-size:14px;font-weight:600;margin-bottom:8px;">Boot failed</div>
            <pre style="white-space:pre-wrap;text-align:left;font-size:11px;background:#fff;padding:12px;border-radius:6px;border:1px solid #e6e3dc;">${escapeHtml(err.message)}\n${escapeHtml(err.stack || "")}</pre>
          </div>
        </div>`);
    });
  }
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", _bootSafely);
  } else {
    _bootSafely();
  }
})();
