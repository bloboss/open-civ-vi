/* ============================================================
 * open4x-units.js — Units & Army screen.
 *
 * Layout:
 *   ┌──────────┬─────────────────────────────────┬─────────┐
 *   │ roster   │ selected unit detail            │ armies  │
 *   │ (units)  │ promotions · actions · combat   │ + form  │
 *   └──────────┴─────────────────────────────────┴─────────┘
 * ============================================================ */
(function () {
  "use strict";

  const state = { units: null, armies: null, selectedId: null };

  async function init() {
    const [u, a] = await Promise.all([api.units(), api.armies()]);
    state.units = u; state.armies = a;
    state.selectedId = (u.units || [])[0]?.id;
    const screen = document.getElementById("screen-units");
    if (!screen) return;
    screen.innerHTML = `
      <div class="units-screen">
        <aside class="units-side scrolly" id="units-side"></aside>
        <section class="units-main scrolly" id="units-main"></section>
        <aside class="units-armies scrolly" id="units-armies"></aside>
      </div>`;
    renderRoster();
    renderDetail();
    renderArmies();
  }

  function getUnit(id) { return (state.units.units || []).find(u => u.id === id); }

  function renderRoster() {
    const root = document.getElementById("units-side");
    const units = state.units.units || [];
    const groups = {};
    for (const u of units) (groups[u.class] = groups[u.class] || []).push(u);
    root.innerHTML = `
      <div class="rail-h"><div class="title">Units</div><div class="meta">${units.length}</div></div>
      <div class="rail-list">
        ${Object.entries(groups).map(([cls, list]) => `
          <div class="rail-group">
            <div class="rail-group-h kicker">${cls}</div>
            ${list.map(u => `
              <button class="rail-item unit-item ${u.id === state.selectedId ? "active" : ""}" data-id="${u.id}">
                <div class="rail-item-l">
                  <div class="rail-item-name">${u.kind}${u.name ? " · " + u.name : ""}</div>
                  <div class="rail-item-meta">${u.terrain} · ${u.position.q},${u.position.r}${u.status === "fortified" ? " · fortified" : ""}</div>
                </div>
                <div class="rail-item-r">
                  <div class="unit-mini-hp">
                    <div class="bar" style="width:36px;"><i style="width:${Math.round(u.hp/u.hp_max*100)}%; background: ${u.hp/u.hp_max > 0.6 ? "var(--good)" : u.hp/u.hp_max > 0.3 ? "var(--warn)" : "var(--bad)"};"></i></div>
                  </div>
                  <div class="num">${u.mp}/${u.mp_max}</div>
                </div>
              </button>
            `).join("")}
          </div>
        `).join("")}
      </div>`;
    root.querySelectorAll(".unit-item").forEach(el => {
      el.addEventListener("click", () => { state.selectedId = el.dataset.id; renderRoster(); renderDetail(); });
    });
  }

  function renderDetail() {
    const root = document.getElementById("units-main");
    const u = getUnit(state.selectedId);
    if (!u) { root.innerHTML = `<div class="prod-empty" style="padding: 40px;">Select a unit</div>`; return; }
    const hpPct = Math.round(u.hp / u.hp_max * 100);
    const xpPct = u.xp_next ? Math.round(u.xp / u.xp_next * 100) : 0;
    root.innerHTML = `
      <div class="unit-hero">
        <div class="unit-hero-l">
          <div class="kicker">${u.class} · ${u.era}</div>
          <h1 class="unit-name">${u.kind}${u.name ? " · " + u.name : ""}</h1>
          <div class="unit-sub">at ${u.position.q},${u.position.r} · ${u.terrain}${u.upkeep_gold ? " · upkeep " + u.upkeep_gold + "G" : ""}</div>
        </div>
        <div class="unit-hero-r">
          <div class="unit-stat-block">
            <div class="kicker">HP</div>
            <div class="num">${u.hp} / ${u.hp_max}</div>
            <div class="bar" style="width:80px;"><i style="width:${hpPct}%; background:${hpPct > 60 ? "var(--good)" : hpPct > 30 ? "var(--warn)" : "var(--bad)"};"></i></div>
          </div>
          <div class="unit-stat-block">
            <div class="kicker">MP</div>
            <div class="num">${u.mp} / ${u.mp_max}</div>
          </div>
          ${u.xp_next ? `<div class="unit-stat-block">
            <div class="kicker">XP</div>
            <div class="num">${u.xp} / ${u.xp_next}</div>
            <div class="bar science" style="width:80px;"><i style="width:${xpPct}%"></i></div>
          </div>` : ""}
        </div>
      </div>

      <div class="unit-stat-row">
        ${u.strength_melee != null ? `<div class="unit-stat"><span class="kicker">Melee</span><span class="num">${u.strength_melee}</span></div>` : ""}
        ${u.strength_ranged != null ? `<div class="unit-stat"><span class="kicker">Ranged</span><span class="num">${u.strength_ranged}</span></div>` : ""}
        <div class="unit-stat"><span class="kicker">Sight</span><span class="num">${u.sight}</span></div>
        ${u.charges_max ? `<div class="unit-stat"><span class="kicker">Charges</span><span class="num">${u.charges} / ${u.charges_max}</span></div>` : ""}
      </div>

      <div class="card">
        <div class="card-h"><span class="title">Actions</span><span class="meta">${(u.actions||[]).filter(a => a.enabled).length} available</span></div>
        <div class="card-b">
          <div class="action-grid">
            ${(u.actions || []).map(a => `
              <button class="action-btn ${!a.enabled ? "disabled" : ""}" ${!a.enabled ? "disabled" : ""}>
                <span class="action-label">${a.label}</span>
                ${a.hotkey ? `<span class="kbd">${a.hotkey}</span>` : ""}
              </button>
            `).join("")}
          </div>
        </div>
      </div>

      ${(u.promotions || []).length ? `
        <div class="card">
          <div class="card-h"><span class="title">Promotions</span><span class="meta">${u.promotions.filter(p => p.chosen).length} chosen</span></div>
          <div class="card-b">
            <div class="promo-grid">
              ${u.promotions.map(p => `
                <div class="promo-card ${p.chosen ? "chosen" : ""} ${p.locked ? "locked" : ""}">
                  <div class="promo-row">
                    <span class="promo-name">${p.name}</span>
                    <span class="promo-lvl">lvl ${p.level || 1}</span>
                  </div>
                  <div class="promo-desc">${p.desc}</div>
                  ${p.chosen ? `<span class="chip good" style="margin-top:6px;">Active</span>` : p.locked ? `<span class="chip warn" style="margin-top:6px;">Locked</span>` : `<button class="btn" style="margin-top:6px;">Take</button>`}
                </div>
              `).join("")}
            </div>
          </div>
        </div>
      ` : ""}
    `;
  }

  function renderArmies() {
    const root = document.getElementById("units-armies");
    const armies = (state.armies?.armies) || [];
    const preview = state.armies?.combat_preview;
    root.innerHTML = `
      <div class="rail-h"><div class="title">Armies</div><div class="meta">${armies.length} / ${state.armies?.corps_slots || "?"}</div></div>
      <div class="rail-body">
        ${armies.map(a => `
          <div class="army-card">
            <div class="army-row"><div class="army-name">${a.name}</div><span class="chip">${a.unit_ids.length} units</span></div>
            <div class="kicker" style="margin: 6px 0 4px;">Cohesion</div>
            <div class="bar accent"><i style="width:${a.cohesion_pct}%"></i></div>
            <div class="army-meta mono">${a.cohesion_pct}%</div>
          </div>
        `).join("") || `<div class="prod-empty">No armies formed</div>`}
        <button class="btn" style="width:100%; margin-top:10px; justify-content:center;">Form army</button>

        ${preview ? `
          <div class="hr" style="margin: 18px 0 12px;"></div>
          <div class="kicker">Combat preview</div>
          <div class="combat-card">
            <div class="combat-row">
              <div>
                <div class="combat-side-label">Attacker</div>
                <div class="combat-side-name">${(state.units.units || []).find(u => u.id === preview.attacker.unit_id)?.kind || "—"}</div>
                <div class="combat-stats mono">STR ${preview.attacker.str} · HP ${preview.attacker.hp}</div>
              </div>
              <div class="combat-vs">VS</div>
              <div style="text-align: right;">
                <div class="combat-side-label">Defender</div>
                <div class="combat-side-name">${preview.defender.name}</div>
                <div class="combat-stats mono">STR ${preview.defender.str} · HP ${preview.defender.hp}</div>
              </div>
            </div>
            <div class="hr" style="margin: 10px 0;"></div>
            <div class="combat-outcome">
              <div class="growth-line"><span class="kicker">Damage taken</span><span class="num">${preview.predicted.dmg_attacker}</span></div>
              <div class="growth-line"><span class="kicker">Damage dealt</span><span class="num pos">${preview.predicted.dmg_defender}</span></div>
              <div class="growth-line" style="margin-top:6px;"><span class="kicker">Outcome</span><span class="chip ${preview.predicted.outcome_label.match(/Win/i) ? "good" : "warn"}">${preview.predicted.outcome_label}</span></div>
            </div>
          </div>
        ` : ""}
      </div>`;
  }

  function repaint() {}
  window.Open4XUnits = { init, repaint };
})();
