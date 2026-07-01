// Outer-loop screens — diplomacy, empire overview, victory, government. All
// read-only displays. (changeGovernment is deferred: no governments-list
// endpoint exists yet — only the current government is returned.)

import type {
  Diplomacy,
  EmpireOverview,
  GovernmentPolicies,
  Victory,
} from "../gen/protocol/protocol.js";
import { clear, el } from "./dom.js";

function rowLine(left: string, right: string, cls = ""): HTMLElement {
  return el("div", { class: `row-line ${cls}`.trim() }, [
    el("span", { text: left }),
    el("span", { class: "muted", text: right }),
  ]);
}

export function renderDiplomacy(host: HTMLElement, d: Diplomacy): void {
  clear(host);
  host.append(el("div", { class: "city-section", text: "Known civilizations" }));
  if (d.civs.length === 0) host.append(el("p", { class: "muted", text: "(none met)" }));
  for (const c of d.civs) {
    host.append(rowLine(`${c.leader} of ${c.name}`, `${c.relation} · ${c.cities_known} cities`));
  }
  if (d.city_states.length > 0) {
    host.append(el("div", { class: "city-section", text: "City-states" }));
    for (const cs of d.city_states) {
      host.append(rowLine(cs.name, `${cs.envoys} envoys${cs.suzerain ? " · suzerain" : ""}`));
    }
  }
}

function statBox(label: string, value: string | number): HTMLElement {
  return el("div", { class: "stat-box" }, [
    el("div", { class: "sb-val", text: String(value) }),
    el("div", { class: "sb-label muted", text: label }),
  ]);
}

export function renderEmpire(host: HTMLElement, e: EmpireOverview): void {
  clear(host);
  const s = e.summary;
  const treasury = `${s.treasury} (${s.treasury_per_turn >= 0 ? "+" : ""}${s.treasury_per_turn})`;
  host.append(
    el("div", { class: "stat-grid" }, [
      statBox("Cities", s.cities),
      statBox("Population", s.population),
      statBox("Treasury", treasury),
      statBox("Military", s.military_units),
    ]),
  );
  host.append(el("div", { class: "city-section", text: "Cities" }));
  for (const c of e.cities) {
    host.append(rowLine(`${c.capital ? "★ " : ""}${c.name}`, `pop ${c.pop}`));
  }
  if (e.strategic_resources.length > 0) {
    host.append(el("div", { class: "city-section", text: "Strategic resources" }));
    for (const r of e.strategic_resources) {
      host.append(rowLine(r.name, `${r.value} (+${r.per_turn})`));
    }
  }
  if (e.luxury_resources.length > 0) {
    host.append(rowLine("Luxuries", e.luxury_resources.join(", ")));
  }
  if (e.trade_routes.length > 0) {
    host.append(
      el("div", { class: "city-section", text: `Trade routes (${e.trade_routes.length}/${e.trade_slots_total})` }),
    );
    for (const t of e.trade_routes) host.append(rowLine(`${t.from} → ${t.to}`, t.yields));
  }
}

function bar(pct: number): HTMLElement {
  const clamped = Math.max(0, Math.min(100, pct));
  const fill = el("div", { class: "bar-fill" });
  fill.style.width = `${clamped}%`;
  return el("div", { class: "bar" }, [fill]);
}

export function renderVictory(host: HTMLElement, v: Victory): void {
  clear(host);
  host.append(
    rowLine(`Rank ${v.rank}/${v.rank_of} · score ${v.score}`, `leading: ${v.leading_condition} ${Math.round(v.leading_pct)}%`),
  );
  host.append(el("div", { class: "city-section", text: "Victory conditions" }));
  for (const c of v.conditions) {
    const pct = Math.round(c.player_pct);
    host.append(
      el("div", { class: "cond-row" }, [
        el("span", { class: "cond-name", text: c.name }),
        bar(pct),
        el("span", { class: "muted", text: `${pct}%` }),
      ]),
    );
  }
  host.append(el("div", { class: "city-section", text: "Leaderboard" }));
  for (const l of v.leaderboard) {
    host.append(rowLine(`${l.rank}. ${l.name}`, String(l.score), l.is_player ? "me" : ""));
  }
}

export function renderGovernment(host: HTMLElement, g: GovernmentPolicies): void {
  clear(host);
  const gov = g.government;
  host.append(el("div", { class: "gov-head", text: `${gov.name} (${gov.era})` }));
  const sl = gov.slots;
  host.append(
    rowLine(`Slots: ${sl.military}M ${sl.economic}E ${sl.diplomatic}D ${sl.wildcard}★`, gov.legacy_bonus),
  );
  host.append(el("div", { class: "city-section", text: "Active policies" }));
  if (g.active_policies.length === 0) host.append(el("p", { class: "muted", text: "(none)" }));
  for (const p of g.active_policies) host.append(rowLine(`[${p.slot}] ${p.name}`, p.effect));

  host.append(el("div", { class: "city-section", text: `Policy catalogue (${g.catalogue.length})` }));
  const cat = el("div", { class: "build-list" });
  for (const c of g.catalogue) {
    cat.append(rowLine(c.name, `${c.type} · ${c.status}`, `status-${c.status}`));
  }
  host.append(cat);
}
