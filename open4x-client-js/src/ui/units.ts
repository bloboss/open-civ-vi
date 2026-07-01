// Unit / Army screen — own-unit list + a detail panel with the unit's available
// actions. Target-less actions (fortify/sleep/found_city) fire immediately;
// move/attack are handed back to GameUI, which captures the next map click as
// the target tile.

import type { Army, Unit } from "../gen/protocol/protocol.js";
import { clear, el } from "./dom.js";

export interface UnitHandlers {
  onSelect: (id: string) => void;
  onAction: (unitId: string, actionId: string) => void;
}

export function renderUnitScreen(
  host: HTMLElement,
  units: Unit[],
  selectedId: string | null,
  armies: Army[],
  h: UnitHandlers,
): void {
  clear(host);
  const own = units.filter((u) => u.is_own);

  const list = el("div", { class: "unit-list" });
  for (const u of own) {
    list.append(
      el(
        "div",
        { class: `unit-li${u.id === selectedId ? " sel" : ""}`, onclick: () => h.onSelect(u.id) },
        [
          el("span", { class: "ui-name", text: u.name }),
          el("span", { class: "muted", text: `${u.kind} · hp ${u.hp}/${u.hp_max} · mp ${u.mp}/${u.mp_max}` }),
        ],
      ),
    );
  }
  if (own.length === 0) list.append(el("p", { class: "muted", text: "No units." }));
  host.append(list);

  const sel = own.find((u) => u.id === selectedId) ?? null;
  if (sel) {
    const panel = el("div", { class: "unit-panel" });
    panel.append(el("div", { class: "unit-head", text: `${sel.name} (${sel.kind})` }));
    const cs = sel.combat_strength === null ? "—" : String(sel.combat_strength);
    panel.append(
      el("div", {
        class: "unit-stats muted",
        text: `hp ${sel.hp}/${sel.hp_max} · mp ${sel.mp}/${sel.mp_max} · cs ${cs} · ${sel.category}/${sel.domain} · (${sel.position.q},${sel.position.r})`,
      }),
    );

    panel.append(el("div", { class: "city-section", text: "Actions" }));
    const acts = el("div", { class: "action-row" });
    for (const a of sel.actions) {
      const btn = el("button", { class: "btn act-btn", text: a.label, onclick: () => h.onAction(sel.id, a.id) });
      if (!a.enabled) (btn as HTMLButtonElement).disabled = true;
      acts.append(btn);
    }
    if (sel.actions.length === 0) acts.append(el("span", { class: "muted", text: "(no actions)" }));
    panel.append(acts);
    host.append(panel);
  }

  if (armies.length > 0) {
    host.append(el("div", { class: "city-section", text: "Armies" }));
    for (const a of armies) {
      host.append(el("div", { class: "muted", text: `${a.name} (${a.units.length} units)` }));
    }
  }
}
