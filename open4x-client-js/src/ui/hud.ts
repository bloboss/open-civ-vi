// HUD resource bar — renders a PlayerState slice. Always-visible top strip.

import type { Bucket, PlayerState } from "../gen/protocol/protocol.js";
import { clear, el } from "./dom.js";

function chip(label: string, b: Bucket): HTMLElement {
  const stored = b.value === null ? "" : `${b.value} `;
  const sign = b.per_turn >= 0 ? "+" : "";
  return el("div", { class: "chip", title: label }, [
    el("span", { class: "chip-label", text: label }),
    el("span", { class: "chip-val", text: `${stored}(${sign}${b.per_turn})` }),
  ]);
}

/** Render the HUD topbar into `host`. `onEndTurn` fires from the End Turn button. */
export function renderHud(host: HTMLElement, ps: PlayerState, onEndTurn: () => void): void {
  clear(host);
  const r = ps.resources;
  const strategics = Object.entries(ps.strategic)
    .filter(([, v]) => (v ?? 0) > 0)
    .map(([k, v]) => `${k} ${v}`)
    .join("  ");

  host.append(
    el("div", { class: "hud-group" }, [
      chip("gold", r.gold),
      chip("sci", r.science),
      chip("cul", r.culture),
      chip("fai", r.faith),
      chip("food", r.food),
      chip("prod", r.production),
    ]),
    el("div", { class: "hud-group hud-mid" }, [
      el("span", { class: "era", text: `${ps.era} ${Math.round(ps.era_progress * 100)}%` }),
      el("span", { class: "happy", text: `happiness ${ps.happiness}` }),
      ...(strategics ? [el("span", { class: "strategic", text: strategics })] : []),
    ]),
    el("div", { class: "hud-group hud-right" }, [
      el("span", { class: "turn", text: `Turn ${ps.turn} / ${ps.turn_max}` }),
      el("button", { class: "btn end-turn", text: "End Turn ▶", onclick: () => onEndTurn() }),
    ]),
  );
}
