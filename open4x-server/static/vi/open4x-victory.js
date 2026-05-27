/* ============================================================
 * open4x-victory.js — Victory conditions & leaderboard.
 * ============================================================ */
(function () {
  "use strict";
  const state = { data: null };

  async function init() {
    state.data = await api.victory();
    const screen = document.getElementById("screen-victory");
    if (!screen) return;
    screen.innerHTML = `<div class="vic-screen scrolly" id="vic-root"></div>`;
    render();
  }

  function render() {
    const d = state.data;
    if (!d) return;
    const root = document.getElementById("vic-root");
    const leadCond = d.conditions.find(c => c.id === d.leading_condition);

    root.innerHTML = `
      <div class="vic-header">
        <div>
          <div class="kicker">Turn ${d.turn} / ${d.turn_max}</div>
          <h1 class="ov-h1">Victory</h1>
          <div class="vic-sub">Projected winner: <strong>${d.projected_winner}</strong></div>
        </div>
        <div class="vic-header-r">
          <div class="vic-rank">
            <div class="kicker">Rank</div>
            <div class="num">#${d.rank} / ${d.rank_of}</div>
          </div>
          <div class="vic-rank">
            <div class="kicker">Score</div>
            <div class="num mono">${d.score?.toLocaleString()}</div>
          </div>
          <div class="vic-rank">
            <div class="kicker">Leading</div>
            <div class="num">${leadCond?.name || "—"} · ${d.leading_pct}%</div>
          </div>
        </div>
      </div>

      <div class="vic-conditions">
        ${d.conditions.map(c => renderCondition(c)).join("")}
      </div>

      <div class="card" style="margin-top: 16px;">
        <div class="card-h"><span class="title">Leaderboard</span><span class="meta">${d.leaderboard.length} civs</span></div>
        <div class="card-b" style="padding: 0;">
          <table class="vic-lb">
            <thead>
              <tr>
                <th>#</th><th>Civ</th><th>Score</th>
                <th>Sci</th><th>Cul</th><th>Dom</th><th>Rel</th><th>Dip</th>
              </tr>
            </thead>
            <tbody>
              ${d.leaderboard.map(r => `
                <tr class="${r.is_player ? "player" : ""}">
                  <td class="mono">${r.rank}</td>
                  <td><strong>${r.name}</strong></td>
                  <td class="mono">${r.score.toLocaleString()}</td>
                  ${["science","culture","domination","religion","diplomatic"].map(k => `
                    <td>
                      <div class="lb-cell">
                        <span class="mono">${r.victory_pcts[k]}%</span>
                        <div class="bar" style="width: 40px; height: 3px;"><i style="width:${r.victory_pcts[k]}%; background: ${vicColor(k)};"></i></div>
                      </div>
                    </td>
                  `).join("")}
                </tr>
              `).join("")}
            </tbody>
          </table>
        </div>
      </div>`;
  }

  function vicColor(id) {
    return ({
      science: "var(--y-sci)",
      culture: "var(--y-cul)",
      domination: "var(--bad)",
      religion: "var(--y-faith)",
      diplomatic: "var(--accent)",
    })[id] || "var(--ink-2)";
  }

  function renderCondition(c) {
    return `
      <div class="card vic-cond">
        <div class="card-h">
          <span class="title">${c.name}</span>
          <span class="num mono" style="color: ${vicColor(c.id)};">${c.player_pct}%</span>
        </div>
        <div class="card-b">
          <div class="bar" style="margin-bottom: 12px;"><i style="width:${c.player_pct}%; background: ${vicColor(c.id)};"></i></div>
          ${c.steps ? `
            <div class="vic-steps">
              ${c.steps.map(s => `
                <div class="vic-step ${s.done ? "done" : ""}">
                  <span class="vic-step-mark">${s.done ? "✓" : "·"}</span>
                  <span>${s.label}</span>
                </div>
              `).join("")}
            </div>
          ` : ""}
          ${c.stats ? `
            <div class="vic-stats">
              ${Object.entries(c.stats).map(([k, v]) => `
                <div class="growth-line"><span class="kicker">${k}</span><span class="num" style="font-size:12px;">${v}</span></div>
              `).join("")}
            </div>
          ` : ""}
        </div>
      </div>`;
  }

  function repaint() {}
  window.Open4XVictory = { init, repaint };
})();
