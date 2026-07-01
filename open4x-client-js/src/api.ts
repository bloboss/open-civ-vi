// Typed client for the open4x `/api/v1` REST surface.
//
// This is the ONLY network layer. It is renderer-agnostic on purpose: the
// WebGL renderer (and any bring-your-own renderer) depends on `Api`, never the
// other way round. All response shapes come from the Rust-generated bindings
// in ./gen/protocol, so the client can never drift from the server contract.

import type {
  ApiErrorBody,
  ArmyData,
  CivicsTreeView,
  CityData,
  CityRow,
  CityTiles,
  CombatPreview,
  Diplomacy,
  EmpireOverview,
  GovernmentPolicies,
  MapOverlays,
  MutationResponse,
  Notifications,
  PlayerState,
  Registry,
  TechTreeView,
  TileView,
  TurnQueue,
  Unit,
  UnitData,
  Victory,
  WorldSnapshot,
} from "./gen/protocol/protocol";

/** Thrown on any non-2xx response, carrying the server's `{error, message}`. */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    readonly detail?: string,
  ) {
    super(`${code} (${status})${detail ? `: ${detail}` : ""}`);
    this.name = "ApiError";
  }
}

/**
 * Response of `POST /api/v1/games/new`. Defined in `open4x-server` (not
 * `open4x-protocol`), so it is not in the generated bundle — mirrored here.
 * TODO(api): move `NewGameResponse` into open4x-protocol so this is generated.
 */
export interface NewGameResponse {
  game_id: string;
  civ_id: string;
  token: string;
  turn: number;
}

/** Body for `POST /api/v1/games/new` (all fields optional; server defaults). */
export interface NewGameRequest {
  width?: number;
  height?: number;
  seed?: number;
  player_civ?: string;
  ai_civs?: string[];
}

/** Body for `POST /api/v1/units/{id}/action`. */
export interface UnitActionBody {
  action_id: string;
  target_q?: number;
  target_r?: number;
  name?: string;
}

/** Body for `POST /api/v1/cities/{id}/production`. */
export interface QueueProductionBody {
  item_id: string;
  item_type: string;
}


/**
 * A bearer-authenticated handle to one game on an `open4x-server`. Reads
 * return the mapped generated type; mutations return the standard
 * `MutationResponse` envelope (`{ ok, view, turn_status }`).
 */
export class Api {
  constructor(
    readonly token: string,
    readonly baseUrl: string = "",
  ) {}

  /**
   * Build an `Api` from a `?token=` query param — the lobby's Resume flow
   * navigates the browser to `<server>/?token=<bearer>`. Returns `null` when
   * no token is present (e.g. a fresh standalone visit).
   */
  static fromLocation(): Api | null {
    const token = new URLSearchParams(globalThis.location.search).get("token");
    return token ? new Api(token) : null;
  }

  /** Bootstrap a fresh single-player session (dev / standalone path). */
  static async bootstrap(body: NewGameRequest = {}, baseUrl = ""): Promise<Api> {
    const res = await request<NewGameResponse>(
      "POST",
      `${baseUrl}/api/v1/games/new`,
      undefined,
      body,
    );
    return new Api(res.token, baseUrl);
  }

  private get<T>(path: string): Promise<T> {
    return request<T>("GET", `${this.baseUrl}/api/v1${path}`, this.token);
  }

  private mutate<T>(method: string, path: string, body?: unknown): Promise<T> {
    return request<T>(method, `${this.baseUrl}/api/v1${path}`, this.token, body);
  }

  // ── reads ────────────────────────────────────────────────────────────────
  playerState(): Promise<PlayerState> {
    return this.get("/player-state");
  }
  worldSnapshot(q = 0, r = 0, radius = 0): Promise<WorldSnapshot> {
    return this.get(`/world/snapshot?q=${q}&r=${r}&radius=${radius}`);
  }
  worldTile(q: number, r: number): Promise<TileView> {
    return this.get(`/world/tile/${q}/${r}`);
  }
  mapOverlays(): Promise<MapOverlays> {
    return this.get("/map/overlays");
  }
  cities(): Promise<CityData> {
    return this.get("/cities");
  }
  city(id: string): Promise<CityRow> {
    return this.get(`/cities/${id}`);
  }
  cityTiles(id: string): Promise<CityTiles> {
    return this.get(`/cities/${id}/tiles`);
  }
  units(): Promise<UnitData> {
    return this.get("/units");
  }
  unit(id: string): Promise<Unit> {
    return this.get(`/units/${id}`);
  }
  armies(): Promise<ArmyData> {
    return this.get("/armies");
  }
  combatPreview(attackerId: string, q: number, r: number): Promise<CombatPreview> {
    return this.get(`/combat/preview?attacker_id=${attackerId}&defender_q=${q}&defender_r=${r}`);
  }
  tech(): Promise<TechTreeView> {
    return this.get("/tech");
  }
  civics(): Promise<CivicsTreeView> {
    return this.get("/civics");
  }
  government(): Promise<GovernmentPolicies> {
    return this.get("/government");
  }
  diplomacy(): Promise<Diplomacy> {
    return this.get("/diplomacy");
  }
  empire(): Promise<EmpireOverview> {
    return this.get("/empire/overview");
  }
  victory(): Promise<Victory> {
    return this.get("/victory");
  }
  registry(): Promise<Registry> {
    return this.get("/registry");
  }
  notifications(): Promise<Notifications> {
    return this.get("/notifications");
  }
  turnQueue(): Promise<TurnQueue> {
    return this.get("/turn-queue");
  }

  // ── mutations (return the `{ ok, view, turn_status }` envelope) ────────────
  endTurn(): Promise<MutationResponse<unknown>> {
    return this.mutate("POST", "/turn/end", {});
  }
  unitAction(id: string, body: UnitActionBody): Promise<MutationResponse<unknown>> {
    return this.mutate("POST", `/units/${id}/action`, body);
  }
  queueProduction(id: string, body: QueueProductionBody): Promise<MutationResponse<unknown>> {
    return this.mutate("POST", `/cities/${id}/production`, body);
  }
  cancelProduction(id: string, pos: number): Promise<MutationResponse<unknown>> {
    return this.mutate("DELETE", `/cities/${id}/production/${pos}`);
  }
  assignCityFocus(id: string, focus: string): Promise<MutationResponse<unknown>> {
    return this.mutate("POST", `/cities/${id}/focus`, { focus });
  }
  renameCity(id: string, name: string): Promise<MutationResponse<unknown>> {
    return this.mutate("POST", `/cities/${id}/rename`, { name });
  }
  setResearch(techId: string): Promise<MutationResponse<unknown>> {
    return this.mutate("POST", "/tech/research", { tech_id: techId });
  }
  cancelResearch(): Promise<MutationResponse<unknown>> {
    return this.mutate("DELETE", "/tech/research");
  }
  setCivic(civicId: string): Promise<MutationResponse<unknown>> {
    return this.mutate("POST", "/civics/research", { civic_id: civicId });
  }
  cancelCivic(): Promise<MutationResponse<unknown>> {
    return this.mutate("DELETE", "/civics/research");
  }
  changeGovernment(governmentId: string): Promise<MutationResponse<unknown>> {
    return this.mutate("POST", "/government/change", { government_id: governmentId });
  }
  dismissNotification(id: string): Promise<void> {
    return this.mutate("DELETE", `/notifications/${id}`);
  }
  dismissAllNotifications(): Promise<void> {
    return this.mutate("DELETE", "/notifications");
  }
}

/**
 * Core fetch helper: attaches the bearer token, JSON-encodes the body, decodes
 * the JSON response, and throws {@link ApiError} on a non-2xx status.
 */
async function request<T>(
  method: string,
  url: string,
  token?: string,
  body?: unknown,
): Promise<T> {
  const headers: Record<string, string> = { accept: "application/json" };
  if (token) headers.authorization = `Bearer ${token}`;
  const init: RequestInit = { method, headers };
  if (body !== undefined) {
    headers["content-type"] = "application/json";
    init.body = JSON.stringify(body);
  }

  const res = await fetch(url, init);
  const text = await res.text();

  if (!res.ok) {
    let code = "http_error";
    let detail: string | undefined;
    try {
      const err = JSON.parse(text) as ApiErrorBody;
      if (err.error) code = err.error;
      detail = err.message ?? undefined;
    } catch {
      // Non-JSON error body — keep the generic code, stash the raw text.
      detail = text || undefined;
    }
    throw new ApiError(res.status, code, detail);
  }

  return (text ? JSON.parse(text) : null) as T;
}
