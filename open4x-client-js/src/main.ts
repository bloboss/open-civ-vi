// open4x-client-js entry — wires the typed Api to the GameUI controller
// (HexMap base layer + HUD topbar + tab panels). Resumes via the lobby's
// `?token=` or bootstraps a standalone session.

import { Api, ApiError } from "./api.js";
import { GameUI } from "./ui/app.js";

function byId<T extends HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) throw new Error(`missing #${id}`);
  return node as T;
}

async function boot(): Promise<void> {
  const status = byId("status");
  let ui: GameUI;
  try {
    ui = new GameUI(
      await resolveApi(),
      byId<HTMLCanvasElement>("gl"),
      byId("hud"),
      byId("tabbar"),
      byId("panel"),
      status,
    );
  } catch (e) {
    status.textContent = `init failed: ${e instanceof ApiError ? e.message : String(e)}`;
    return;
  }

  try {
    await ui.load();
  } catch (e) {
    status.textContent = `load failed: ${e instanceof ApiError ? e.message : String(e)}`;
  }
}

async function resolveApi(): Promise<Api> {
  return (
    Api.fromLocation() ??
    (await Api.bootstrap({
      width: 24,
      height: 16,
      seed: 7,
      player_civ: "Rome",
      ai_civs: ["Babylon"],
    }))
  );
}

void boot();
