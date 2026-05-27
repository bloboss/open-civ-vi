/* ============================================================
 * open4x-civics.js — Civics tree screen.
 *
 * Same shape as open4x-tech.js but reads civics data and adds
 * a current-government summary to the side panel.
 * ============================================================ */
(function () {
  "use strict";

  const state = { tree: null, govt: null, selected: null };

  const COL_W   = 200;
  const NODE_W  = 168;
  const NODE_H  = 64;
  const ROW_GAP = 14;
  const HEADER_H = 36;
  const ROW_MAX = 8;

  async function init() {
    state.tree = await api.civics();
    state.govt = await api.government();
    layoutTree();
    const screen = document.getElementById("screen-civics");
    if (!screen) return;
    screen.innerHTML = `
      <div class="civics-screen">
        <section class="civics-canvas-wrap scrolly" id="civics-canvas-wrap">
          <div class="civics-canvas tech-canvas" id="civics-canvas"></div>
        </section>
        <aside class="civics-side tech-side scrolly" id="civics-side"></aside>
      </div>
    `;
    render();
    renderSide();
  }

  function layoutTree() {
    const techs = state.tree.civics || [];
    const eras  = state.tree.era_columns || [];
    const eraIdx = {};
    eras.forEach((e, i) => { eraIdx[e.id] = i; });
    const byEra = eras.map(() => []);
    for (const t of techs) {
      const i = eraIdx[t.era];
      if (i == null) continue;
      byEra[i].push(t);
    }
    const posById = new Map();
    for (let c = 0; c < byEra.length; c++) {
      const col = byEra[c];
      col.sort((a, b) => bestRowHint(a, posById) - bestRowHint(b, posById));
      const usedRows = new Set();
      for (const t of col) {
        let hint = bestRowHint(t, posById);
        let row = Math.max(0, Math.min(ROW_MAX - 1, Math.round(hint)));
        while (usedRows.has(row) && row < ROW_MAX - 1) row++;
        while (usedRows.has(row) && row > 0) row--;
        if (usedRows.has(row)) row = usedRows.size;
        usedRows.add(row);
        posById.set(t.id, { col: c, row });
      }
    }
    state.pos = posById;
    state.rows = Math.max(...[...posById.values()].map(p => p.row), 0) + 1;
  }
  function bestRowHint(tech, posById) {
    const prs = tech.prereqs || [];
    if (!prs.length) return 1.5;
    const rows = prs.map(id => posById.get(id)?.row).filter(r => r != null);
    if (!rows.length) return 1.5;
    return rows.reduce((a, b) => a + b, 0) / rows.length;
  }

  function render() {
    const root = document.getElementById("civics-canvas");
    const eras = state.tree.era_columns || [];
    const civics = state.tree.civics || [];
    const totalW = eras.length * COL_W;
    const totalH = HEADER_H + (state.rows * (NODE_H + ROW_GAP)) + 30;
    root.style.width  = totalW + "px";
    root.style.height = totalH + "px";

    let edgesSvg = `<svg class="tech-edges" width="${totalW}" height="${totalH}" xmlns="http://www.w3.org/2000/svg">`;
    for (const t of civics) {
      for (const pid of (t.prereqs || [])) {
        const a = state.pos.get(pid);
        const b = state.pos.get(t.id);
        if (!a || !b) continue;
        const ax = a.col * COL_W + NODE_W + 4;
        const ay = HEADER_H + a.row * (NODE_H + ROW_GAP) + NODE_H / 2;
        const bx = b.col * COL_W;
        const by = HEADER_H + b.row * (NODE_H + ROW_GAP) + NODE_H / 2;
        const mid = (ax + bx) / 2;
        const prereq = civics.find(x => x.id === pid);
        const done = prereq?.status === "done";
        const cls = done ? "tech-edge done" : "tech-edge";
        edgesSvg += `<path class="${cls}" d="M ${ax} ${ay} C ${mid} ${ay} ${mid} ${by} ${bx} ${by}" />`;
      }
    }
    edgesSvg += `</svg>`;

    const headers = eras.map((e, i) => `
      <div class="civic-era-h tech-era-h" style="left: ${i * COL_W}px;">
        <div class="kicker">${e.id}</div>
        <div class="era-bar" style="width:${COL_W - 28}px;">
          <i style="width:${100 / eras.length * (i + 1)}%; background: var(--y-cul);"></i>
        </div>
      </div>
    `).join("");

    const cards = civics.map(t => {
      const p = state.pos.get(t.id);
      if (!p) return "";
      const x = p.col * COL_W + 8;
      const y = HEADER_H + p.row * (NODE_H + ROW_GAP);
      const cur = state.tree.civic_queue || [];
      const qIdx = cur.indexOf(t.id);
      const inQ = qIdx >= 0;
      const pctNum = t.cost ? Math.round((t.progress || 0) / t.cost * 100) : 0;
      return `
        <button class="tech-card civic-card ${t.status} ${inQ ? "queued" : ""}" data-id="${t.id}"
                style="left:${x}px; top:${y}px; width:${NODE_W}px; height:${NODE_H}px;">
          <div class="tech-card-h">
            <span class="tech-name">${t.name}</span>
            ${inQ ? `<span class="tech-q-pos" style="background: #f0deef; color: var(--y-cul);">${qIdx + 1}</span>` : ""}
            ${t.status === "done" ? `<span class="tech-check">✓</span>` : ""}
            ${t.status === "current" ? `<span class="tech-current-pct mono" style="color: var(--y-cul);">${pctNum}%</span>` : ""}
          </div>
          <div class="tech-meta">
            <span class="tech-cost mono">${t.cost}<span class="tech-cost-u">C</span></span>
            <span class="tech-unlocks">${t.unlocks}</span>
          </div>
          ${t.status === "current" ? `<div class="bar culture tech-progress"><i style="width:${pctNum}%"></i></div>` : ""}
        </button>
      `;
    }).join("");

    root.innerHTML = edgesSvg + headers + cards;

    root.querySelectorAll(".tech-card").forEach(el => {
      el.addEventListener("click", () => {
        state.selected = el.dataset.id;
        renderSide();
        root.querySelectorAll(".tech-card").forEach(x => x.classList.toggle("selected", x.dataset.id === state.selected));
      });
    });
  }

  function renderSide() {
    const root = document.getElementById("civics-side");
    const civics = state.tree.civics || [];
    const cur = civics.find(t => t.status === "current");
    const queueIds = state.tree.civic_queue || [];
    const queue = queueIds.map(id => civics.find(t => t.id === id)).filter(Boolean);
    const sel = state.selected ? civics.find(t => t.id === state.selected) : null;
    const g = state.tree.government || state.govt?.current_government || {};

    root.innerHTML = `
      <div class="tech-side-section">
        <div class="kicker">Currently developing</div>
        ${cur ? `
          <div class="tech-current-card" style="background: #f6ecf5; border-color: #ead9e8;">
            <div class="tech-current-name" style="color: var(--y-cul);">${cur.name}</div>
            <div class="tech-current-meta mono">${cur.progress || 0} / ${cur.cost} C · ${Math.round((cur.progress||0)/cur.cost*100)}%</div>
            <div class="bar culture" style="margin-top:8px;"><i style="width:${Math.round((cur.progress||0)/cur.cost*100)}%"></i></div>
            <div class="tech-current-unlocks">Unlocks: <strong>${cur.unlocks}</strong></div>
            <div class="tech-current-eta mono">~${Math.ceil((cur.cost - (cur.progress||0)) / 32)} turns</div>
          </div>
        ` : `<div class="prod-empty">No active civic</div>`}
      </div>

      <div class="tech-side-section">
        <div class="kicker">Queue · ${queue.length}</div>
        <div class="tech-queue">
          ${queue.map((q, i) => `
            <div class="tech-q-row">
              <span class="tech-q-num">${i + 1}</span>
              <span class="tech-q-name">${q.name}</span>
              <span class="tech-q-cost mono">${q.cost}C</span>
              <button class="btn ghost tech-q-rm" data-id="${q.id}" title="Remove">×</button>
            </div>
          `).join("") || `<div class="prod-empty">Queue empty</div>`}
        </div>
      </div>

      <div class="tech-side-section">
        <div class="kicker">Current government</div>
        <div class="govt-summary">
          <div class="govt-name">${g.name || "—"}</div>
          ${g.slots ? `
            <div class="govt-slots">
              ${Object.entries(g.slots).map(([k, v]) => v > 0 ? `
                <div class="govt-slot">
                  <span class="govt-slot-k">${k.slice(0,3)}</span>
                  <span class="govt-slot-v mono">${v}</span>
                </div>
              ` : "").join("")}
            </div>
          ` : ""}
          ${(g.policies || []).length ? `
            <div class="govt-policies">
              ${g.policies.map(p => `
                <div class="govt-policy">
                  <span class="govt-policy-name">${p.name}</span>
                  <span class="govt-policy-effect mono">${p.effect}</span>
                </div>
              `).join("")}
            </div>
          ` : ""}
          <button class="btn" style="margin-top: 12px; width: 100%; justify-content: center;"
                  onclick="document.querySelector('[data-screen=&quot;govt&quot;]').click()">
            Manage government
          </button>
        </div>
      </div>

      ${sel ? `
        <div class="tech-side-section">
          <div class="kicker">Selected</div>
          <div class="tech-sel-card">
            <div class="tech-sel-name">${sel.name}</div>
            <div class="tech-sel-meta">
              <span class="chip ${selChip(sel.status)}">${sel.status}</span>
              <span class="mono">${sel.cost}C</span>
              <span style="color:var(--ink-4);">·</span>
              <span>${sel.era}</span>
            </div>
            <div class="tech-sel-unlocks"><span class="kicker">Unlocks</span> ${sel.unlocks}</div>
            ${(sel.prereqs || []).length ? `
              <div class="tech-sel-prereqs">
                <span class="kicker">Prereqs</span>
                ${sel.prereqs.map(pid => {
                  const p = civics.find(t => t.id === pid);
                  return `<span class="chip ${p && p.status === "done" ? "good" : ""}">${p?.name || pid}</span>`;
                }).join(" ")}
              </div>
            ` : ""}
            <div class="tech-sel-actions">
              ${sel.status === "available" ? `
                <button class="btn-primary" data-act="research">Develop</button>
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
        if (b.dataset.act === "research") await api.civicResearch(state.selected);
        if (b.dataset.act === "queue")    await api.civicQueue(state.selected);
      });
    });
  }
  function selChip(status) {
    if (status === "done") return "good";
    if (status === "current") return "accent";
    if (status === "locked") return "warn";
    return "";
  }
  function repaint() {}

  window.Open4XCivics = { init, repaint };
})();
