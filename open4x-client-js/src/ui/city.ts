// City management screen — own-city list + a detail panel with rename, citizen
// focus, the production queue, and a build picker. The `/cities` + `/cities/{id}`
// endpoints both return the flat `CityRow` (queue = item-name strings, focus =
// lowercase). The buildable catalogue comes from `registry()` (units +
// buildings); the REST surface has no per-city "available production" today, so
// some entries may be tech-gated and 400 on queue (handled by the caller).

import type { CityFocus, CityRow, Registry } from "../gen/protocol/protocol.js";
import { clear, el } from "./dom.js";

const FOCI: CityFocus[] = [
  "Default",
  "Food",
  "Production",
  "Gold",
  "Science",
  "Culture",
  "Faith",
];

export interface CityHandlers {
  onSelect: (id: string) => void;
  onRename: (id: string, name: string) => void;
  onFocus: (id: string, focus: CityFocus) => void;
  onQueue: (id: string, itemId: string, itemType: string) => void;
  onCancel: (id: string, pos: number) => void;
}

function buildRow(name: string, cost: number, onClick: () => void): HTMLElement {
  return el("div", { class: "build-row clickable", onclick: onClick }, [
    el("span", { class: "b-name", text: name }),
    el("span", { class: "muted", text: `${cost}` }),
  ]);
}

export function renderCityScreen(
  host: HTMLElement,
  cities: CityRow[],
  selected: CityRow | null,
  registry: Registry,
  h: CityHandlers,
): void {
  clear(host);
  const own = cities.filter((c) => c.is_own);

  const list = el("div", { class: "city-list" });
  for (const c of own) {
    list.append(
      el(
        "div",
        { class: `city-li${selected?.id === c.id ? " sel" : ""}`, onclick: () => h.onSelect(c.id) },
        [
          el("span", { class: "ci-name", text: `${c.capital ? "★ " : ""}${c.name}` }),
          el("span", { class: "muted", text: `pop ${c.population}` }),
        ],
      ),
    );
  }
  host.append(list);

  if (!selected) {
    host.append(el("p", { class: "muted", text: own.length ? "Select a city." : "No cities yet — found one with a settler." }));
    return;
  }

  const c = selected;
  const id = c.id;
  const panel = el("div", { class: "city-panel" });

  const nameInput = el("input", { class: "input" }) as HTMLInputElement;
  nameInput.value = c.name;
  panel.append(
    el("div", { class: "city-head" }, [
      nameInput,
      el("button", { class: "btn", text: "rename", onclick: () => h.onRename(id, nameInput.value) }),
    ]),
  );

  panel.append(
    el("div", {
      class: "city-stats muted",
      text: `pop ${c.population} · food ${c.food_stored}/${c.food_to_grow} · prod ${c.production_stored} · focus ${c.focus}`,
    }),
  );

  panel.append(el("div", { class: "city-section", text: "Focus" }));
  const focusRow = el("div", { class: "focus-row" });
  for (const f of FOCI) {
    const active = c.focus === f.toLowerCase() ? " active" : "";
    focusRow.append(
      el("button", { class: `btn focus-btn${active}`, text: f, onclick: () => h.onFocus(id, f) }),
    );
  }
  panel.append(focusRow);

  panel.append(el("div", { class: "city-section", text: "Production queue" }));
  if (c.production_queue.length === 0) {
    panel.append(el("p", { class: "muted", text: "(idle)" }));
  }
  c.production_queue.forEach((name, pos) => {
    panel.append(
      el("div", { class: "queue-row" }, [
        el("span", { text: `${pos === 0 ? "▸ " : ""}${name}` }),
        el("button", { class: "btn", text: "✕", title: "cancel", onclick: () => h.onCancel(id, pos) }),
      ]),
    );
  });

  panel.append(el("div", { class: "city-section", text: "Build (full catalogue)" }));
  const builds = el("div", { class: "build-list" });
  for (const u of registry.unit_types) {
    builds.append(buildRow(u.name, u.production_cost, () => h.onQueue(id, u.id, "unit")));
  }
  for (const b of registry.buildings) {
    builds.append(buildRow(b.name, b.cost, () => h.onQueue(id, b.id, "building")));
  }
  panel.append(builds);

  host.append(panel);
}
