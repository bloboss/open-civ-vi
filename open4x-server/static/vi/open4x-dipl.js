/* ============================================================
 * open4x-dipl.js — Diplomacy screen.
 *
 * Layout:
 *   ┌────────┬─────────────────────┬─────────────┐
 *   │ civs + │ active civ detail   │ deal draft  │
 *   │ city   │ relation modifiers  │ + city-st.  │
 *   │ states │ treaties · actions  │             │
 *   └────────┴─────────────────────┴─────────────┘
 * ============================================================ */
(function () {
  "use strict";

  const state = { dipl: null, activeId: null };

  async function init() {
    state.dipl = await api.diplomacy();
    state.activeId = state.dipl.active_civ || state.dipl.civs[0]?.id;
    const screen = document.getElementById("screen-dipl");
    if (!screen) return;
    screen.innerHTML = `
      <div class="dipl-screen">
        <aside class="dipl-side scrolly" id="dipl-side"></aside>
        <section class="dipl-main scrolly" id="dipl-main"></section>
        <aside class="dipl-deal scrolly" id="dipl-deal"></aside>
      </div>`;
    renderList();
    renderDetail();
    renderDeal();
  }

  function getCiv(id) { return (state.dipl.civs || []).find(c => c.id === id); }

  function relCls(r) {
    if (r === "Friendly") return "good";
    if (r === "Unfriendly") return "warn";
    if (r === "At War") return "bad";
    return "";
  }

  function renderList() {
    const root = document.getElementById("dipl-side");
    const civs = state.dipl.civs || [];
    const states = state.dipl.city_states || [];
    root.innerHTML = `
      <div class="rail-h"><div class="title">Civilizations</div><div class="meta">${civs.length}</div></div>
      <div class="rail-list">
        ${civs.map(c => `
          <button class="rail-item civ-item ${c.id === state.activeId ? "active" : ""}" data-id="${c.id}">
            <div class="rail-item-l">
              <div class="rail-item-name">${c.name}</div>
              <div class="rail-item-meta">${c.leader} · ${c.agenda}</div>
            </div>
            <div class="rail-item-r">
              <span class="chip ${relCls(c.relation)}">${c.relation}</span>
              ${c.relation_score !== 0 ? `<div class="num" style="font-size:11px; color:${c.relation_score > 0 ? "var(--good)" : "var(--bad)"};">${c.relation_score > 0 ? "+" : ""}${c.relation_score}</div>` : ""}
            </div>
          </button>
        `).join("")}
      </div>

      <div class="rail-h" style="margin-top: 8px;"><div class="title">City-states</div><div class="meta">${states.length}</div></div>
      <div class="rail-list">
        ${states.map(cs => `
          <div class="rail-item cs-item">
            <div class="rail-item-l">
              <div class="rail-item-name">${cs.name} ${cs.suzerain ? "★" : ""}</div>
              <div class="rail-item-meta">${cs.bonus || ""}</div>
            </div>
            <div class="rail-item-r">
              <span class="chip">${cs.envoys}🤝</span>
            </div>
          </div>
        `).join("")}
      </div>`;
    root.querySelectorAll(".civ-item").forEach(el => {
      el.addEventListener("click", () => { state.activeId = el.dataset.id; renderList(); renderDetail(); renderDeal(); });
    });
  }

  function renderDetail() {
    const root = document.getElementById("dipl-main");
    const c = getCiv(state.activeId);
    if (!c) { root.innerHTML = `<div class="prod-empty" style="padding:40px;">Select a civilization</div>`; return; }
    root.innerHTML = `
      <div class="dipl-hero">
        <div class="dipl-hero-l">
          <div class="kicker">${c.government}</div>
          <h1 class="dipl-name">${c.name}</h1>
          <div class="dipl-sub">${c.leader} · ${c.agenda}</div>
        </div>
        <div class="dipl-hero-r">
          <div class="dipl-rel">
            <span class="chip ${relCls(c.relation)}" style="font-size:12px; padding:3px 12px;">${c.relation}</span>
            ${c.relation_score !== 0 ? `<div class="rel-score num ${c.relation_score > 0 ? "pos" : "neg"}">${c.relation_score > 0 ? "+" : ""}${c.relation_score}</div>` : ""}
          </div>
        </div>
      </div>

      <div class="dipl-stats">
        <div class="dipl-stat"><span class="kicker">Cities known</span><span class="num">${c.cities_known}</span></div>
        <div class="dipl-stat"><span class="kicker">Military</span><span class="num">${c.military_est}</span></div>
        <div class="dipl-stat"><span class="kicker">Science</span><span class="num">${c.science_est}</span></div>
        <div class="dipl-stat"><span class="kicker">Trade routes</span><span class="num">${c.trade_routes}${c.trade_yields ? " · " + c.trade_yields : ""}</span></div>
      </div>

      ${(c.treaties || []).length ? `
        <div class="card">
          <div class="card-h"><span class="title">Active treaties</span></div>
          <div class="card-b">
            <div style="display: flex; gap: 6px; flex-wrap: wrap;">
              ${c.treaties.map(t => `<span class="chip accent">${t}</span>`).join("")}
            </div>
          </div>
        </div>
      ` : ""}

      <div class="card">
        <div class="card-h"><span class="title">Relation modifiers</span><span class="meta">${c.modifiers.length}</span></div>
        <div class="card-b">
          ${(c.modifiers || []).length ? c.modifiers.map(m => `
            <div class="mod-row mod-${m.kind}">
              <span class="mod-dot ${m.kind === "good" ? "good" : "bad"}"></span>
              <span class="mod-desc">${m.desc}</span>
              <span class="mod-val num ${m.kind === "good" ? "pos" : "neg"}">${m.value > 0 ? "+" : ""}${m.value}</span>
            </div>
          `).join("") : `<div class="prod-empty">No active modifiers</div>`}
        </div>
      </div>

      <div class="card">
        <div class="card-h"><span class="title">Actions</span></div>
        <div class="card-b">
          <div class="dipl-actions">
            ${c.relation !== "At War" ? `
              <button class="btn">Open borders</button>
              <button class="btn">Research agreement</button>
              <button class="btn">Alliance</button>
              <button class="btn">Send delegation</button>
              <button class="btn danger">Denounce</button>
              <button class="btn danger">Declare war</button>
            ` : `
              <button class="btn">Make peace</button>
              <button class="btn danger">Continue war</button>
            `}
          </div>
        </div>
      </div>`;
  }

  function renderDeal() {
    const root = document.getElementById("dipl-deal");
    const c = getCiv(state.activeId);
    const draft = state.dipl.deal_draft || {};
    root.innerHTML = `
      <div class="rail-h"><div class="title">Deal draft</div><div class="meta">${c ? c.name : "—"}</div></div>
      <div class="rail-body">
        <div class="deal-side">
          <div class="kicker">You give</div>
          <div class="deal-items">
            ${(draft.you_give || []).map(i => `<div class="deal-item">${i} <button class="deal-rm">×</button></div>`).join("") || `<div class="prod-empty">Nothing offered</div>`}
          </div>
          <button class="btn ghost" style="width: 100%; margin-top: 6px;">+ Add offer</button>
        </div>
        <div class="hr" style="margin: 12px 0;"></div>
        <div class="deal-side">
          <div class="kicker">They give</div>
          <div class="deal-items">
            ${(draft.they_give || []).map(i => `<div class="deal-item">${i} <button class="deal-rm">×</button></div>`).join("") || `<div class="prod-empty">Nothing requested</div>`}
          </div>
          <button class="btn ghost" style="width: 100%; margin-top: 6px;">+ Add request</button>
        </div>
        <div class="hr" style="margin: 12px 0;"></div>
        <div class="growth-line"><span class="kicker">Balance</span><span class="chip ${draft.balance === "fair" ? "good" : ""}">${draft.balance || "—"}</span></div>
        <div class="growth-line" style="margin-top:6px;"><span class="kicker">AI mood</span><span class="chip">${draft.mood || "—"}</span></div>
        <div class="growth-line" style="margin-top:6px;"><span class="kicker">Duration</span><span class="mono">${draft.duration_turns || "?"} turns</span></div>
        <button class="btn-primary" style="width: 100%; margin-top: 16px; justify-content: center;">Propose deal</button>
      </div>`;
  }

  function repaint() {}
  window.Open4XDipl = { init, repaint };
})();
