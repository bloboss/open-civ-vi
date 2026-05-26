/* ============================================================
 * open4x-govt.js — Government & Policies screen.
 * Layout: current gov + slots top, policy catalogue grid below.
 * ============================================================ */
(function () {
  "use strict";
  const state = { data: null, tab: "all" };

  async function init() {
    state.data = await api.government();
    const screen = document.getElementById("screen-govt");
    if (!screen) return;
    screen.innerHTML = `<div class="govt-screen scrolly" id="govt-root"></div>`;
    render();
  }

  function render() {
    const root = document.getElementById("govt-root");
    const d = state.data;
    if (!d) return;
    const g = d.government || {};
    root.innerHTML = `
      <div class="govt-hero">
        <div>
          <div class="kicker">Government · ${g.era || ""}</div>
          <h1 class="govt-name-h1">${g.name}</h1>
          <div class="govt-legacy">${g.legacy_bonus || ""}</div>
        </div>
        <div class="govt-slot-counts">
          ${Object.entries(g.slots || {}).map(([k, v]) => `
            <div class="slot-count slot-${k}">
              <div class="num">${v}</div>
              <div class="kicker">${k}</div>
            </div>
          `).join("")}
        </div>
      </div>

      <div class="card">
        <div class="card-h"><span class="title">Active policies</span><span class="meta">${(d.active_policies || []).length} slotted</span></div>
        <div class="card-b">
          ${renderSlots(g.slots || {}, d.active_policies || [])}
        </div>
      </div>

      <div class="card" style="margin-top: 16px;">
        <div class="card-h">
          <span class="title">Policy catalogue</span>
          <div class="catalogue-tabs">
            ${["all","military","economic","diplomatic","wildcard"].map(k => `
              <button class="cat-tab ${state.tab === k ? "active" : ""}" data-tab="${k}">${k}</button>
            `).join("")}
          </div>
        </div>
        <div class="card-b">
          <div class="policy-grid">
            ${(d.catalogue || [])
              .filter(p => state.tab === "all" || p.type === state.tab)
              .map(p => `
                <div class="policy-card policy-${p.type} policy-${p.status}">
                  <div class="policy-h">
                    <span class="policy-name">${p.name}</span>
                    <span class="chip policy-type">${p.type}</span>
                  </div>
                  <div class="policy-effect">${p.effect}</div>
                  <div class="policy-foot">
                    <span class="kicker">${p.era}${p.dark_age ? " · dark" : ""}</span>
                    <span class="chip ${p.status === "active" ? "good" : p.status === "locked" ? "warn" : "accent"}">${p.status}</span>
                  </div>
                </div>
              `).join("")}
          </div>
        </div>
      </div>

      <div class="card" style="margin-top: 16px;">
        <div class="card-h"><span class="title">Available governments</span></div>
        <div class="card-b">
          <div class="govt-list">
            ${(d.available_governments || []).map(gv => `
              <div class="govt-row ${gv.current ? "current" : ""} ${gv.locked ? "locked" : ""}">
                <div class="govt-row-l">
                  <div class="govt-row-name">${gv.name} ${gv.current ? "★" : ""}</div>
                  <div class="govt-row-meta">${gv.era}${gv.unlock_civic ? " · req " + gv.unlock_civic : ""}</div>
                </div>
                <div class="govt-row-slots">
                  ${Object.entries(gv.slots).map(([k, v]) => v > 0 ? `<span class="slot-mini slot-${k}">${v}</span>` : "").join("")}
                </div>
                <div>
                  ${gv.current ? `<span class="chip good">Current</span>` :
                    gv.locked ? `<span class="chip warn">Locked</span>` :
                    `<button class="btn">Switch · 1 turn anarchy</button>`}
                </div>
              </div>
            `).join("")}
          </div>
        </div>
      </div>`;

    root.querySelectorAll(".cat-tab").forEach(b => {
      b.addEventListener("click", () => { state.tab = b.dataset.tab; render(); });
    });
  }

  function renderSlots(slots, active) {
    const buckets = { military: [], economic: [], diplomatic: [], wildcard: [] };
    for (const p of active) (buckets[p.slot] || (buckets[p.slot] = [])).push(p);
    const types = ["military","economic","diplomatic","wildcard"];
    return `<div class="slot-rows">
      ${types.map(t => {
        const max = slots[t] || 0;
        const cards = buckets[t] || [];
        const empties = Math.max(0, max - cards.length);
        if (max === 0 && cards.length === 0) return "";
        return `
          <div class="slot-row slot-${t}">
            <div class="slot-row-label">
              <span class="kicker">${t}</span>
              <span class="mono">${cards.length}/${max}</span>
            </div>
            <div class="slot-cards">
              ${cards.map(c => `
                <div class="slot-card-mini">
                  <span class="slot-card-name">${c.name}</span>
                  <span class="slot-card-effect">${c.effect}</span>
                </div>
              `).join("")}
              ${Array.from({length: empties}, () => `<div class="slot-card-empty">empty slot</div>`).join("")}
            </div>
          </div>`;
      }).join("")}
    </div>`;
  }

  function repaint() {}
  window.Open4XGovt = { init, repaint };
})();
