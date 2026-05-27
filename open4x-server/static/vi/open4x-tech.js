/* ============================================================
 * open4x-tech.js — Technology tree screen.
 *
 * Layout:
 *   ┌──────────────────────────────────┬─────────────────┐
 *   │ era columns w/ nodes + edges     │ active research │
 *   │ (horizontal scroll)              │ + queue + focus │
 *   └──────────────────────────────────┴─────────────────┘
 *
 * Each tech node is rendered as a small card with:
 *   - name + cost
 *   - unlocks (small caps)
 *   - status border (built / current / available / locked)
 *   - prereq lines drawn behind via an SVG layer
 * ============================================================ */
(function () {
  "use strict";

  const state = {
    tree: null,
    selected: null,
  };

  const COL_W   = 200;   // era column width
  const NODE_W  = 168;   // tech card width
  const NODE_H  = 64;    // tech card height
  const ROW_GAP = 14;    // vertical gap between cards
  const HEADER_H = 36;   // era header row

  // dim cap: tree is ~28 techs across 6 eras, so 6 rows tall max
  const ROW_MAX = 8;

  async function init() {
    state.tree = await api.tech();
    layoutTree();
    const screen = document.getElementById("screen-tech");
    if (!screen) return;
    screen.innerHTML = `
      <div class="tech-screen">
        <section class="tech-canvas-wrap scrolly" id="tech-canvas-wrap">
          <div class="tech-canvas" id="tech-canvas"></div>
        </section>
        <aside class="tech-side scrolly" id="tech-side"></aside>
      </div>
    `;
    render();
    renderSide();
  }

  // ── place techs in (era column × row) grid ─────────────
  function layoutTree() {
    const techs = state.tree.techs || [];
    const eras  = state.tree.era_columns || [];
    const eraIdx = {};
    eras.forEach((e, i) => { eraIdx[e.id] = i; });

    // group by era, then assign rows trying to respect prereqs
    const byEra = eras.map(() => []);
    for (const t of techs) {
      const i = eraIdx[t.era];
      if (i == null) continue;
      byEra[i].push(t);
    }
    // assign row index per era, kept consistent with prereq's row when possible
    const posById = new Map();
    for (let c = 0; c < byEra.length; c++) {
      const col = byEra[c];
      // sort: roots first, then by prereq row hint
      col.sort((a, b) => {
        const ar = bestRowHint(a, posById);
        const br = bestRowHint(b, posById);
        return ar - br;
      });
      const usedRows = new Set();
      for (const t of col) {
        let hint = bestRowHint(t, posById);
        let row = Math.max(0, Math.min(ROW_MAX - 1, Math.round(hint)));
        while (usedRows.has(row) && row < ROW_MAX - 1) row++;
        while (usedRows.has(row) && row > 0) row--;
        if (usedRows.has(row)) row = usedRows.size; // last resort
        usedRows.add(row);
        posById.set(t.id, { col: c, row });
      }
    }
    state.pos = posById;
    state.rows = Math.max(...[...posById.values()].map(p => p.row)) + 1;
  }

  function bestRowHint(tech, posById) {
    const prs = tech.prereqs || [];
    if (!prs.length) return 1.5;  // root center
    const rows = prs.map(id => posById.get(id)?.row).filter(r => r != null);
    if (!rows.length) return 1.5;
    return rows.reduce((a, b) => a + b, 0) / rows.length;
  }

  // ── render ─────────────────────────────────────────────
  function render() {
    const root = document.getElementById("tech-canvas");
    const eras = state.tree.era_columns || [];
    const techs = state.tree.techs || [];

    const totalW = eras.length * COL_W;
    const totalH = HEADER_H + (state.rows * (NODE_H + ROW_GAP)) + 30;
    root.style.width  = totalW + "px";
    root.style.height = totalH + "px";

    // edges first (SVG behind cards)
    let edgesSvg = `<svg class="tech-edges" width="${totalW}" height="${totalH}" xmlns="http://www.w3.org/2000/svg">`;
    for (const t of techs) {
      for (const pid of (t.prereqs || [])) {
        const a = state.pos.get(pid);
        const b = state.pos.get(t.id);
        if (!a || !b) continue;
        const ax = a.col * COL_W + NODE_W + 4;
        const ay = HEADER_H + a.row * (NODE_H + ROW_GAP) + NODE_H / 2;
        const bx = b.col * COL_W;
        const by = HEADER_H + b.row * (NODE_H + ROW_GAP) + NODE_H / 2;
        const mid = (ax + bx) / 2;
        const target = techs.find(x => x.id === t.id);
        const prereq = techs.find(x => x.id === pid);
        const done = prereq?.status === "done";
        const cls = done ? "tech-edge done" : "tech-edge";
        edgesSvg += `<path class="${cls}" d="M ${ax} ${ay} C ${mid} ${ay} ${mid} ${by} ${bx} ${by}" />`;
      }
    }
    edgesSvg += `</svg>`;

    // era headers
    const headers = eras.map((e, i) => `
      <div class="tech-era-h" style="left: ${i * COL_W}px;">
        <div class="kicker">${e.id}</div>
        <div class="era-bar" style="width:${COL_W - 28}px;">
          <i style="width:${100 / eras.length * (i + 1)}%"></i>
        </div>
      </div>
    `).join("");

    // tech cards
    const cards = techs.map(t => {
      const p = state.pos.get(t.id);
      if (!p) return "";
      const x = p.col * COL_W + 8;
      const y = HEADER_H + p.row * (NODE_H + ROW_GAP);
      const cur = state.tree.research_queue || [];
      const qIdx = cur.indexOf(t.id);
      const inQ = qIdx >= 0;
      const pctNum = t.cost ? Math.round((t.progress || 0) / t.cost * 100) : 0;
      return `
        <button class="tech-card ${t.status} ${inQ ? "queued" : ""}" data-id="${t.id}"
                style="left:${x}px; top:${y}px; width:${NODE_W}px; height:${NODE_H}px;">
          <div class="tech-card-h">
            <span class="tech-name">${t.name}</span>
            ${inQ ? `<span class="tech-q-pos">${qIdx + 1}</span>` : ""}
            ${t.status === "done" ? `<span class="tech-check">✓</span>` : ""}
            ${t.status === "current" ? `<span class="tech-current-pct mono">${pctNum}%</span>` : ""}
          </div>
          <div class="tech-meta">
            <span class="tech-cost mono">${t.cost}<span class="tech-cost-u">S</span></span>
            <span class="tech-unlocks">${t.unlocks}</span>
          </div>
          ${t.status === "current" ? `<div class="bar science tech-progress"><i style="width:${pctNum}%"></i></div>` : ""}
        </button>
      `;
    }).join("");

    root.innerHTML = edgesSvg + headers + cards;

    root.querySelectorAll(".tech-card").forEach(el => {
      el.addEventListener("click", () => {
        state.selected = el.dataset.id;
        renderSide();
        // visually highlight
        root.querySelectorAll(".tech-card").forEach(x => x.classList.toggle("selected", x.dataset.id === state.selected));
      });
    });
  }

  // ── right side ─────────────────────────────────────────
  function renderSide() {
    const root = document.getElementById("tech-side");
    const techs = state.tree.techs || [];
    const cur = techs.find(t => t.status === "current");
    const queueIds = state.tree.research_queue || [];
    const queue = queueIds.map(id => techs.find(t => t.id === id)).filter(Boolean);

    // selected detail
    const sel = state.selected ? techs.find(t => t.id === state.selected) : null;

    root.innerHTML = `
      <div class="tech-side-section">
        <div class="kicker">Currently researching</div>
        ${cur ? `
          <div class="tech-current-card">
            <div class="tech-current-name">${cur.name}</div>
            <div class="tech-current-meta mono">${cur.progress || 0} / ${cur.cost} S · ${Math.round((cur.progress||0)/cur.cost*100)}%</div>
            <div class="bar science" style="margin-top:8px;"><i style="width:${Math.round((cur.progress||0)/cur.cost*100)}%"></i></div>
            <div class="tech-current-unlocks">Unlocks: <strong>${cur.unlocks}</strong></div>
            <div class="tech-current-eta mono">~${Math.ceil((cur.cost - (cur.progress||0)) / 47)} turns</div>
          </div>
        ` : `<div class="prod-empty">No active research</div>`}
      </div>

      <div class="tech-side-section">
        <div class="kicker">Queue · ${queue.length}</div>
        <div class="tech-queue">
          ${queue.map((q, i) => `
            <div class="tech-q-row">
              <span class="tech-q-num">${i + 1}</span>
              <span class="tech-q-name">${q.name}</span>
              <span class="tech-q-cost mono">${q.cost}S</span>
              <button class="btn ghost tech-q-rm" data-id="${q.id}" title="Remove">×</button>
            </div>
          `).join("") || `<div class="prod-empty">Queue empty</div>`}
        </div>
      </div>

      ${sel ? `
        <div class="tech-side-section">
          <div class="kicker">Selected</div>
          <div class="tech-sel-card">
            <div class="tech-sel-name">${sel.name}</div>
            <div class="tech-sel-meta">
              <span class="chip ${selChip(sel.status)}">${sel.status}</span>
              <span class="mono">${sel.cost}S</span>
              <span style="color:var(--ink-4);">·</span>
              <span>${sel.era}</span>
            </div>
            <div class="tech-sel-unlocks"><span class="kicker">Unlocks</span> ${sel.unlocks}</div>
            ${(sel.prereqs || []).length ? `
              <div class="tech-sel-prereqs">
                <span class="kicker">Prereqs</span>
                ${sel.prereqs.map(pid => {
                  const p = techs.find(t => t.id === pid);
                  return `<span class="chip ${p && p.status === "done" ? "good" : ""}">${p?.name || pid}</span>`;
                }).join(" ")}
              </div>
            ` : ""}
            <div class="tech-sel-actions">
              ${sel.status === "available" ? `
                <button class="btn-primary" data-act="research">Research</button>
                <button class="btn" data-act="queue">Queue</button>
              ` : ""}
              ${sel.status === "current" ? `<button class="btn" data-act="cancel">Cancel</button>` : ""}
              ${sel.status === "locked" ? `<span class="prod-empty" style="padding:6px 0;">Locked — complete prereqs first</span>` : ""}
              ${sel.status === "done" ? `<span class="chip good">Completed</span>` : ""}
            </div>
          </div>
        </div>
      ` : ""}
    `;

    root.querySelectorAll("[data-act]").forEach(b => {
      b.addEventListener("click", async () => {
        if (!state.selected) return;
        if (b.dataset.act === "research") await api.research(state.selected);
        if (b.dataset.act === "queue")    await api.techQueue(state.selected);
        // optimistic local update: refetch + re-render would be nicer
      });
    });
  }

  function selChip(status) {
    if (status === "done") return "good";
    if (status === "current") return "accent";
    if (status === "available") return "";
    if (status === "locked") return "warn";
    return "";
  }

  function repaint() {
    // tech tree is DOM-based so no special canvas handling needed
  }

  window.Open4XTech = { init, repaint };
})();
