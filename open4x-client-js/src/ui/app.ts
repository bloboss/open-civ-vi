// GameUI — top-level controller. Owns the HexMap (base layer), the HUD topbar,
// a tab panel, and a status line. Holds the typed `Api` and a `refresh()` that
// re-pulls slices after a mutation. Screen panels (tech/civics/city/units/…)
// are filled in by later sub-slices; 8a ships the shell + HUD + End Turn.

import { Api, ApiError } from "../api.js";
import { HexMap } from "../render/hexmap.js";
import { type CityHandlers, renderCityScreen } from "./city.js";
import { clear, el } from "./dom.js";
import { renderNotifications, renderOverlays, renderTurnQueue } from "./drawers.js";
import { renderHud } from "./hud.js";
import { renderDiplomacy, renderEmpire, renderGovernment, renderVictory } from "./outer.js";
import { renderTree } from "./trees.js";
import { type UnitHandlers, renderUnitScreen } from "./units.js";

/** Unit actions that require a target tile (captured via the next map click). */
const TARGETED_ACTIONS = new Set(["move", "attack"]);

type DrawerKind = "notifications" | "turnq" | "overlays";
const DRAWERS: [DrawerKind, string][] = [
  ["notifications", "🔔 notif"],
  ["turnq", "⏳ queue"],
  ["overlays", "▦ overlays"],
];

export type Tab =
  | "tech"
  | "civics"
  | "city"
  | "units"
  | "diplomacy"
  | "empire"
  | "victory"
  | "government";

const TABS: Tab[] = ["tech", "civics", "city", "units", "diplomacy", "empire", "victory", "government"];

export class GameUI {
  private readonly map: HexMap;
  private activeTab: Tab | null = null;
  private selectedCity: string | null = null;
  private selectedUnit: string | null = null;
  private pendingAction: { unitId: string; actionId: string } | null = null;
  private readonly drawerBar: HTMLElement;
  private readonly drawerEl: HTMLElement;
  private activeDrawer: DrawerKind | null = null;
  private readonly overlayIds = new Set<string>();
  private overlaysSeeded = false;

  constructor(
    private readonly api: Api,
    canvas: HTMLCanvasElement,
    private readonly hudEl: HTMLElement,
    private readonly tabbarEl: HTMLElement,
    private readonly panelEl: HTMLElement,
    private readonly statusEl: HTMLElement,
  ) {
    this.map = new HexMap(canvas);
    this.map.onPick = (q, r) => {
      if (this.pendingAction) {
        const { unitId, actionId } = this.pendingAction;
        void this.execUnitAction(unitId, actionId, q, r);
      } else {
        this.setStatus(`tile (${q}, ${r})`);
      }
    };
    this.renderTabbar();

    // HUD drawers (created here so main.ts/index.html stay untouched).
    this.drawerBar = el("div", { class: "drawer-bar" });
    this.drawerEl = el("div", { class: "drawer" });
    document.body.append(this.drawerBar, this.drawerEl);
    this.renderDrawerBar();
  }

  private setStatus(msg: string): void {
    this.statusEl.textContent = msg;
  }

  /** Initial load: map + entities + HUD. */
  async load(): Promise<void> {
    const snap = await this.api.worldSnapshot(0, 0, 0);
    this.map.setSnapshot(snap);
    const [cities, units] = await Promise.all([this.api.cities(), this.api.units()]);
    this.map.setEntities(cities.cities, units.units);
    this.map.draw();
    await this.refreshHud();
    this.setStatus(`${snap.tiles.length} tiles · turn ${snap.world.turn} · drag/zoom/click the map`);
  }

  private async refreshHud(): Promise<void> {
    const ps = await this.api.playerState();
    renderHud(this.hudEl, ps, () => void this.endTurn());
  }

  private async endTurn(): Promise<void> {
    try {
      const res = await this.api.endTurn();
      this.setStatus(`turn ended → now turn ${res.turn_status.turn}`);
      await this.load(); // turn advanced — refetch the world.
    } catch (e) {
      if (e instanceof ApiError && e.code === "unresolved_required_actions") {
        // Surface the blocking items from the turn queue.
        const tq = await this.api.turnQueue();
        const required = tq.items.filter((i) => i.required).map((i) => i.title);
        this.setStatus(`Can't end turn — resolve: ${required.join(", ") || "required actions"}`);
      } else {
        this.setStatus(`end turn failed: ${e instanceof ApiError ? e.message : String(e)}`);
      }
    }
  }

  private renderTabbar(): void {
    clear(this.tabbarEl);
    for (const tab of TABS) {
      this.tabbarEl.append(
        el("button", {
          class: "tab",
          text: tab,
          onclick: () => void this.showTab(tab),
        }),
      );
    }
  }

  /** Toggle a tab's panel and load its content. */
  private async showTab(tab: Tab): Promise<void> {
    this.activeTab = this.activeTab === tab ? null : tab;
    for (const b of this.tabbarEl.querySelectorAll("button.tab")) {
      b.classList.toggle("active", b.textContent === this.activeTab);
    }
    clear(this.panelEl);
    if (!this.activeTab) {
      this.panelEl.classList.remove("open");
      return;
    }
    this.panelEl.classList.add("open");
    const body = el("div", { class: "panel-body" });
    this.panelEl.append(el("div", { class: "panel-head", text: this.activeTab.toUpperCase() }), body);
    await this.loadPanel(this.activeTab, body);
  }

  /** Fetch + render a tab's data. 8b wires tech/civics; others stay placeholders. */
  private async loadPanel(tab: Tab, body: HTMLElement): Promise<void> {
    try {
      if (tab === "tech") {
        const t = await this.api.tech();
        renderTree(body, t.techs, t.research_queue, (id) => void this.pickResearch(id));
      } else if (tab === "civics") {
        const c = await this.api.civics();
        renderTree(body, c.civics, c.civic_queue, (id) => void this.pickCivic(id));
      } else if (tab === "city") {
        await this.loadCity(body);
      } else if (tab === "units") {
        await this.loadUnits(body);
      } else if (tab === "diplomacy") {
        renderDiplomacy(body, await this.api.diplomacy());
      } else if (tab === "empire") {
        renderEmpire(body, await this.api.empire());
      } else if (tab === "victory") {
        renderVictory(body, await this.api.victory());
      } else if (tab === "government") {
        renderGovernment(body, await this.api.government());
      }
    } catch (e) {
      body.append(el("p", { class: "muted", text: `failed: ${this.errMsg(e)}` }));
    }
  }

  /** Re-run the active tab's loader (after a mutation). */
  private async refreshPanel(): Promise<void> {
    if (!this.activeTab) return;
    const body = this.panelEl.querySelector(".panel-body");
    if (body instanceof HTMLElement) await this.loadPanel(this.activeTab, body);
  }

  private async pickResearch(id: string): Promise<void> {
    try {
      await this.api.setResearch(id);
    } catch (e) {
      this.setStatus(`research failed: ${this.errMsg(e)}`);
      return;
    }
    this.setStatus("research queued");
    await Promise.all([this.refreshHud(), this.refreshPanel()]);
  }

  private async pickCivic(id: string): Promise<void> {
    try {
      await this.api.setCivic(id);
    } catch (e) {
      this.setStatus(`civic failed: ${this.errMsg(e)}`);
      return;
    }
    this.setStatus("civic queued");
    await Promise.all([this.refreshHud(), this.refreshPanel()]);
  }

  // ── city screen ────────────────────────────────────────────────────────────
  private async loadCity(body: HTMLElement): Promise<void> {
    const [data, registry] = await Promise.all([this.api.cities(), this.api.registry()]);
    const own = data.cities.filter((c) => c.is_own);
    if (!this.selectedCity || !own.some((c) => c.id === this.selectedCity)) {
      this.selectedCity = own[0]?.id ?? null;
    }
    const selected = data.cities.find((c) => c.id === this.selectedCity) ?? null;
    renderCityScreen(body, data.cities, selected, registry, this.cityHandlers());
  }

  private cityHandlers(): CityHandlers {
    return {
      onSelect: (id) => {
        this.selectedCity = id;
        void this.refreshPanel();
      },
      onRename: (id, name) => void this.cityMutate(() => this.api.renameCity(id, name), "renamed"),
      onFocus: (id, focus) =>
        void this.cityMutate(() => this.api.assignCityFocus(id, focus), `focus → ${focus}`),
      onQueue: (id, itemId, itemType) =>
        void this.cityMutate(
          () => this.api.queueProduction(id, { item_id: itemId, item_type: itemType }),
          "queued",
        ),
      onCancel: (id, pos) =>
        void this.cityMutate(() => this.api.cancelProduction(id, pos), "cancelled"),
    };
  }

  private async cityMutate(fn: () => Promise<unknown>, okMsg: string): Promise<void> {
    try {
      await fn();
    } catch (e) {
      this.setStatus(`failed: ${this.errMsg(e)}`);
      return;
    }
    this.setStatus(okMsg);
    await Promise.all([this.refreshHud(), this.refreshPanel()]);
  }

  // ── unit / army screen ───────────────────────────────────────────────────
  private async loadUnits(body: HTMLElement): Promise<void> {
    const [u, ar] = await Promise.all([this.api.units(), this.api.armies()]);
    const own = u.units.filter((x) => x.is_own);
    if (!this.selectedUnit || !own.some((x) => x.id === this.selectedUnit)) {
      this.selectedUnit = own[0]?.id ?? null;
    }
    renderUnitScreen(body, u.units, this.selectedUnit, ar.armies, this.unitHandlers());
  }

  private unitHandlers(): UnitHandlers {
    return {
      onSelect: (id) => {
        this.selectedUnit = id;
        void this.refreshPanel();
      },
      onAction: (unitId, actionId) => {
        if (TARGETED_ACTIONS.has(actionId)) {
          this.pendingAction = { unitId, actionId };
          this.setStatus(`${actionId}: click a target tile`);
        } else {
          void this.execUnitAction(unitId, actionId);
        }
      },
    };
  }

  private async execUnitAction(
    unitId: string,
    actionId: string,
    q?: number,
    r?: number,
  ): Promise<void> {
    const payload: { action_id: string; target_q?: number; target_r?: number } = {
      action_id: actionId,
    };
    if (q !== undefined) payload.target_q = q;
    if (r !== undefined) payload.target_r = r;
    try {
      await this.api.unitAction(unitId, payload);
    } catch (e) {
      this.setStatus(`${actionId} failed: ${this.errMsg(e)}`);
      return;
    }
    this.pendingAction = null;
    this.setStatus(`${actionId} done`);
    await this.refreshMap();
    await Promise.all([this.refreshHud(), this.refreshPanel()]);
  }

  /** Refresh unit/city markers without recentering the camera. */
  private async refreshMap(): Promise<void> {
    const [cities, units] = await Promise.all([this.api.cities(), this.api.units()]);
    this.map.setEntities(cities.cities, units.units);
    this.map.draw();
  }

  // ── HUD drawers ──────────────────────────────────────────────────────────
  private renderDrawerBar(): void {
    clear(this.drawerBar);
    for (const [kind, label] of DRAWERS) {
      this.drawerBar.append(
        el("button", {
          class: `drawer-btn${this.activeDrawer === kind ? " active" : ""}`,
          text: label,
          onclick: () => void this.showDrawer(kind),
        }),
      );
    }
  }

  private async showDrawer(kind: DrawerKind): Promise<void> {
    this.activeDrawer = this.activeDrawer === kind ? null : kind;
    this.renderDrawerBar();
    if (!this.activeDrawer) {
      this.drawerEl.classList.remove("open");
      return;
    }
    this.drawerEl.classList.add("open");
    await this.loadDrawer();
  }

  private async loadDrawer(): Promise<void> {
    try {
      if (this.activeDrawer === "notifications") {
        const data = await this.api.notifications();
        renderNotifications(this.drawerEl, data, {
          onDismiss: (id) => void this.drawerMutate(() => this.api.dismissNotification(id)),
          onClearAll: () => void this.drawerMutate(() => this.api.dismissAllNotifications()),
        });
      } else if (this.activeDrawer === "turnq") {
        renderTurnQueue(this.drawerEl, await this.api.turnQueue());
      } else if (this.activeDrawer === "overlays") {
        const data = await this.api.mapOverlays();
        if (!this.overlaysSeeded) {
          for (const o of data.overlays) if (o.active) this.overlayIds.add(o.id);
          this.overlaysSeeded = true;
        }
        renderOverlays(this.drawerEl, data, this.overlayIds, {
          onToggle: (id) => {
            if (this.overlayIds.has(id)) this.overlayIds.delete(id);
            else this.overlayIds.add(id);
            void this.loadDrawer(); // local-only re-render (renderer apply is a follow-up)
          },
        });
      }
    } catch (e) {
      clear(this.drawerEl);
      this.drawerEl.append(el("p", { class: "muted", text: `failed: ${this.errMsg(e)}` }));
    }
  }

  private async drawerMutate(fn: () => Promise<unknown>): Promise<void> {
    try {
      await fn();
    } catch (e) {
      this.setStatus(`failed: ${this.errMsg(e)}`);
      return;
    }
    await this.loadDrawer();
  }

  private errMsg(e: unknown): string {
    return e instanceof ApiError ? e.message : String(e);
  }
}
