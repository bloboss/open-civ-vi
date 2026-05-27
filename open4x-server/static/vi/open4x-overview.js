/* ============================================================
 * open4x-overview.js — Empire Overview dashboard.
 *
 * Top: summary stat blocks (cities/pop/treasury/military)
 * Middle: cities table + resources panel
 * Bottom: trade routes + religion + yields sparkline
 * ============================================================ */
(function () {
  "use strict";
  const state = { data: null };

  async function init() {
    state.data = await api.empireOverview();
    const screen = document.getElementById("screen-overview");
    if (!screen) return;
    screen.innerHTML = `<div class="ov-screen scrolly" id="ov-root"></div>`;
    render();
  }

  function render() {
    const d = state.data;
    if (!d) return;
    const s = d.summary || {};
    const root = document.getElementById("ov-root");

    root.innerHTML = `
      <div class="ov-header">
        <div>
          <div class="kicker">Empire overview</div>
          <h1 class="ov-h1">Egypt</h1>
        </div>
      </div>

      <div class="ov-summary">
        <div class="ov-stat">
          <div class="kicker">Cities</div>
          <div class="ov-stat-num">${s.cities}</div>
          <div class="ov-stat-meta">
            <span class="chip good">${s.cities_happy} happy</span>
            <span class="chip">${s.cities_content} content</span>
            ${s.cities_unhappy ? `<span class="chip bad">${s.cities_unhappy} unhappy</span>` : ""}
          </div>
        </div>
        <div class="ov-stat">
          <div class="kicker">Population</div>
          <div class="ov-stat-num">${s.population}</div>
          <div class="ov-stat-meta mono">+${s.pop_growth_avg} avg / turn</div>
        </div>
        <div class="ov-stat">
          <div class="kicker">Treasury</div>
          <div class="ov-stat-num mono">${s.treasury?.toLocaleString()}</div>
          <div class="ov-stat-meta">
            <span class="chip ${s.treasury_per_turn > 0 ? "good" : "bad"}">${s.treasury_per_turn > 0 ? "+" : ""}${s.treasury_per_turn}/turn</span>
            ${s.treasury_bankrupt_turns ? `<span class="chip warn">~${s.treasury_bankrupt_turns}t to bankrupt</span>` : ""}
          </div>
        </div>
        <div class="ov-stat">
          <div class="kicker">Military</div>
          <div class="ov-stat-num">${s.military_units}</div>
          <div class="ov-stat-meta">
            <span class="mono">str ${s.military_strength}</span>
            <span class="chip">#${s.military_rank} / ${s.military_rank_of}</span>
          </div>
        </div>
      </div>

      <div class="ov-grid">
        <div class="card ov-cities-card">
          <div class="card-h"><span class="title">Cities</span><span class="meta">${d.cities.length}</span></div>
          <div class="card-b" style="padding: 0;">
            <table class="ov-cities-tbl">
              <thead>
                <tr>
                  <th>Name</th><th>Pop</th><th>Producing</th><th>Yields</th><th>Hsg</th><th>Amen</th><th>Loy</th>
                </tr>
              </thead>
              <tbody>
                ${d.cities.map(c => `
                  <tr class="${c.loyalty_warn ? "warn" : ""}">
                    <td><strong>${c.name}</strong>${c.capital ? " ★" : ""}</td>
                    <td class="mono">${c.pop}</td>
                    <td>${c.production_item} <span class="mono">${c.production_turns}t</span></td>
                    <td><span class="yields">
                      <span class="y y-f">${c.yields.f}</span>
                      <span class="y y-p">${c.yields.p}</span>
                      <span class="y y-g">${c.yields.g}</span>
                      <span class="y y-s">${c.yields.sci}</span>
                      <span class="y y-c">${c.yields.cul}</span>
                    </span></td>
                    <td class="mono">${c.housing.used}/${c.housing.max}</td>
                    <td class="mono ${c.amenities < 0 ? "neg" : c.amenities > 0 ? "pos" : ""}">${c.amenities >= 0 ? "+" : ""}${c.amenities}</td>
                    <td class="mono ${c.loyalty_warn ? "neg" : ""}">${c.loyalty_pct}%</td>
                  </tr>
                `).join("")}
              </tbody>
            </table>
          </div>
        </div>

        <div class="card ov-resources-card">
          <div class="card-h"><span class="title">Strategic</span></div>
          <div class="card-b">
            ${(d.strategic_resources || []).map(r => `
              <div class="growth-line ${r.era_locked ? "locked" : ""}">
                <span>${r.name}</span>
                <span class="num">${r.value}${r.per_turn ? ` <span style="color:var(--ink-4); font-size:10px;">+${r.per_turn}/t</span>` : ""}</span>
              </div>
            `).join("")}
          </div>
          <div class="card-h" style="border-top: 1px solid var(--border-soft);"><span class="title">Luxury</span></div>
          <div class="card-b">
            <div style="display: flex; flex-wrap: wrap; gap: 4px;">
              ${(d.luxury_resources || []).map(l => `<span class="chip">${l}</span>`).join("")}
            </div>
          </div>
        </div>

        <div class="card ov-trade-card">
          <div class="card-h"><span class="title">Trade routes</span><span class="meta">${(d.trade_routes||[]).length} / ${d.trade_slots_total}</span></div>
          <div class="card-b">
            ${(d.trade_routes || []).map(t => `
              <div class="trade-row">
                <div class="trade-route">${t.from} → ${t.to}</div>
                <div class="trade-yields mono">${t.yields}</div>
                ${t.turns_total ? `<div class="trade-bar"><div class="bar" style="width:100%;"><i style="width:${t.turns_done/t.turns_total*100}%"></i></div><span class="mono">${t.turns_done}/${t.turns_total}t</span></div>` : `<div class="trade-bar"><span class="chip">internal</span></div>`}
              </div>
            `).join("")}
          </div>
        </div>

        <div class="card ov-religion-card">
          <div class="card-h"><span class="title">Religion</span></div>
          <div class="card-b">
            <div class="growth-line"><span>Pantheon</span><span>${d.religion?.pantheon || "—"}</span></div>
            <div class="growth-line"><span>World religion</span><span>${d.religion?.founded ? "founded" : "not founded"}</span></div>
            <div class="kicker" style="margin: 10px 0 6px;">Great Prophet</div>
            <div class="mono">${d.religion?.prophet_progress || 0} / ${d.religion?.prophet_needed || 4}</div>
            <div class="bar faith" style="margin-top: 4px;"><i style="width:${(d.religion?.prophet_progress / d.religion?.prophet_needed * 100) || 0}%"></i></div>
            ${d.religion?.foreign_in_cities ? `<div class="growth-line" style="margin-top: 10px;"><span class="kicker">Foreign</span><span style="font-size: 11px;">${d.religion.foreign_in_cities}</span></div>` : ""}
          </div>
        </div>

        <div class="card ov-chart-card">
          <div class="card-h"><span class="title">Yields trend</span><span class="meta">last ${d.yields_chart?.length || 0} samples</span></div>
          <div class="card-b">
            ${renderSpark(d.yields_chart || [])}
          </div>
        </div>
      </div>`;
  }

  function renderSpark(samples) {
    if (!samples.length) return "";
    const W = 600, H = 140, padL = 32, padR = 8, padT = 10, padB = 24;
    const innerW = W - padL - padR, innerH = H - padT - padB;
    const max = Math.max(...samples.map(s => Math.max(s.science, s.culture, s.gold))) * 1.05;
    const x = i => padL + (i / (samples.length - 1)) * innerW;
    const y = v => padT + innerH - (v / max) * innerH;
    function path(key) {
      return samples.map((s, i) => `${i === 0 ? "M" : "L"} ${x(i).toFixed(1)} ${y(s[key]).toFixed(1)}`).join(" ");
    }
    const series = [
      { key: "science", color: "var(--y-sci)", label: "Science" },
      { key: "culture", color: "var(--y-cul)", label: "Culture" },
      { key: "gold",    color: "var(--y-gold)", label: "Gold" },
    ];
    const lastTurn = samples[samples.length - 1].turn;
    const firstTurn = samples[0].turn;
    return `
      <svg width="100%" viewBox="0 0 ${W} ${H}" preserveAspectRatio="xMidYMid meet" style="display:block;">
        <line x1="${padL}" y1="${y(max)}" x2="${W-padR}" y2="${y(max)}" stroke="var(--border-soft)" stroke-dasharray="2 3"/>
        <line x1="${padL}" y1="${y(max*0.5)}" x2="${W-padR}" y2="${y(max*0.5)}" stroke="var(--border-soft)" stroke-dasharray="2 3"/>
        <line x1="${padL}" y1="${y(0)}" x2="${W-padR}" y2="${y(0)}" stroke="var(--border)"/>
        <text x="${padL-4}" y="${y(max)+4}" text-anchor="end" font-size="10" fill="var(--ink-4)" font-family="var(--font-mono)">${Math.round(max)}</text>
        <text x="${padL-4}" y="${y(0)+4}" text-anchor="end" font-size="10" fill="var(--ink-4)" font-family="var(--font-mono)">0</text>
        <text x="${padL}" y="${H-4}" text-anchor="start" font-size="10" fill="var(--ink-4)" font-family="var(--font-mono)">T${firstTurn}</text>
        <text x="${W-padR}" y="${H-4}" text-anchor="end" font-size="10" fill="var(--ink-4)" font-family="var(--font-mono)">T${lastTurn}</text>
        ${series.map(s => `<path d="${path(s.key)}" fill="none" stroke="${s.color}" stroke-width="1.5"/>`).join("")}
        ${series.map((s, i) => `
          <g transform="translate(${padL + i*84}, ${H - 4})">
            <circle cx="-8" cy="-4" r="3" fill="${s.color}"/>
            <text x="-2" y="0" font-size="10" fill="var(--ink-3)">${s.label}</text>
          </g>`).join("")}
      </svg>`;
  }

  function repaint() {}
  window.Open4XOverview = { init, repaint };
})();
