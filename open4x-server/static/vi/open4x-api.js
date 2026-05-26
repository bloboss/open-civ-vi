/* ============================================================
 * open4x-api.js — data adapter for the hi-fi 4X UI.
 *
 * Wraps every REST endpoint specified in api-manifest.json.
 *
 * Single switching point:
 *   api.config.mode = "json"     → fetch the bundled JSON fixtures
 *   api.config.mode = "live"     → fetch /api/v1/* with bearer token
 *
 * Public surface:
 *   await api.playerState()              → /player-state
 *   await api.worldSnapshot(q, r, rad)   → /world/snapshot
 *   await api.tile(q, r)                 → /world/tile/:q/:r
 *   await api.mapOverlays()              → /map/overlays
 *   await api.units()                    → /units
 *   await api.unit(id)                   → /units/:id
 *   await api.armies()                   → /armies
 *   await api.combatPreview(...)         → /combat/preview
 *   await api.cities()                   → /cities
 *   await api.city(id)                   → /cities/:id
 *   await api.cityTiles(id)              → /cities/:id/tiles
 *   await api.tech()                     → /tech
 *   await api.civics()                   → /civics
 *   await api.government()               → /government
 *   await api.diplomacy()                → /diplomacy
 *   await api.empireOverview()           → /empire/overview
 *   await api.victory()                  → /victory
 *   await api.notifications()            → /notifications
 *   await api.turnQueue()                → /turn-queue
 *
 *   await api.unitAction(id, body)       → POST /units/:id/action
 *   await api.endTurn()                  → POST /turn/end
 *   await api.queueProduction(...)       → POST /cities/:id/production
 *   await api.cancelProduction(...)      → DELETE
 *   await api.research(techId)           → POST /tech/research
 *   await api.civicResearch(civicId)     → POST /civics/research
 *   await api.toggleOverlay(id, active)  → POST /map/overlays/:id/toggle
 *   await api.dismissNotification(id)    → DELETE
 *   await api.skipQueueItem(id)          → POST /turn-queue/:id/skip
 *
 * Every method returns a Promise.
 * In "json" mode write operations are no-ops that resolve with
 *   { ok: true, mock: true }.
 * ============================================================ */
(function () {
  "use strict";

  const config = {
    mode: "json",          // "json" | "live"
    baseUrl: "/api/v1",
    token: null,
  };

  // ── local JSON fixture map ─────────────────────────────
  const FIXTURES = {
    "player-state":     "./player-state.json",
    "world-snapshot":   "./world-snapshot.json",
    "map-overlays":     "./map-overlays.json",
    "units":            "./unit-data.json",
    "armies":           "./army-data.json",
    "cities":           "./city-data.json",
    "city-tiles":       "./city-tiles.json",
    "tech":             "./tech-tree.json",
    "civics":           "./civics-tree.json",
    "government":       "./government-policies.json",
    "diplomacy":        "./diplomacy.json",
    "empire-overview":  "./empire-overview.json",
    "victory":          "./victory.json",
    "notifications":    "./notifications.json",
    "turn-queue":       "./turn-queue.json",
  };

  // module-scoped cache so the same fixture is parsed once.
  const _cache = new Map();

  async function _fetchJson(url) {
    if (_cache.has(url)) return _cache.get(url);
    const r = await fetch(url, { cache: "no-cache" });
    if (!r.ok) throw new Error(`fetch ${url}: ${r.status}`);
    const data = await r.json();
    _cache.set(url, data);
    return data;
  }

  async function _liveGet(path) {
    const headers = { Accept: "application/json" };
    if (config.token) headers.Authorization = `Bearer ${config.token}`;
    const r = await fetch(`${config.baseUrl}${path}`, { headers });
    if (!r.ok) {
      const txt = await r.text().catch(() => "");
      throw new Error(`GET ${path}: ${r.status} ${txt}`);
    }
    return r.json();
  }
  async function _liveSend(method, path, body) {
    const headers = { "Content-Type": "application/json", Accept: "application/json" };
    if (config.token) headers.Authorization = `Bearer ${config.token}`;
    const r = await fetch(`${config.baseUrl}${path}`, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
    });
    if (!r.ok) {
      const txt = await r.text().catch(() => "");
      throw new Error(`${method} ${path}: ${r.status} ${txt}`);
    }
    return r.json().catch(() => ({ ok: true }));
  }

  // ── reads ─────────────────────────────────────────────
  function _read(key) {
    return async (..._args) => {
      if (config.mode === "live") {
        return _liveGet(_argsToPath(key, _args));
      }
      return _fetchJson(FIXTURES[key]);
    };
  }
  function _argsToPath(key, args) {
    switch (key) {
      case "world-snapshot": {
        const [q, r, radius] = args;
        const p = new URLSearchParams();
        if (q != null) p.set("q", q);
        if (r != null) p.set("r", r);
        if (radius != null) p.set("radius", radius);
        return `/world/snapshot?${p}`;
      }
      case "tile":            return `/world/tile/${args[0]}/${args[1]}`;
      case "unit":            return `/units/${args[0]}`;
      case "city":            return `/cities/${args[0]}`;
      case "city-tiles":      return `/cities/${args[0]}/tiles`;
      case "diplomacy-civ":   return `/diplomacy/civs/${args[0]}`;
      case "combat-preview": {
        const [attacker_id, target_q, target_r] = args;
        const p = new URLSearchParams();
        p.set("attacker_id", attacker_id);
        p.set("defender_q", target_q);
        p.set("defender_r", target_r);
        return `/combat/preview?${p}`;
      }
      default:                return "/" + key.replace(/_/g, "-");
    }
  }

  // ── writes ────────────────────────────────────────────
  async function _write(method, livePath, body) {
    if (config.mode === "live") return _liveSend(method, livePath, body);
    return Promise.resolve({ ok: true, mock: true, body });
  }

  // ── public surface ────────────────────────────────────
  const api = {
    config,

    // simple reads
    playerState:     _read("player-state"),
    worldSnapshot:   _read("world-snapshot"),
    mapOverlays:     _read("map-overlays"),
    units:           _read("units"),
    armies:          _read("armies"),
    cities:          _read("cities"),
    tech:            _read("tech"),
    civics:          _read("civics"),
    government:      _read("government"),
    diplomacy:       _read("diplomacy"),
    empireOverview:  _read("empire-overview"),
    victory:         _read("victory"),
    notifications:   _read("notifications"),
    turnQueue:       _read("turn-queue"),

    // parameterised reads
    async tile(q, r) {
      if (config.mode === "live") return _liveGet(`/world/tile/${q}/${r}`);
      const w = await _fetchJson(FIXTURES["world-snapshot"]);
      return (w.tiles || []).find(t => t.q === q && t.r === r) || null;
    },
    async unit(id) {
      if (config.mode === "live") return _liveGet(`/units/${id}`);
      const u = await _fetchJson(FIXTURES.units);
      return (u.units || []).find(x => x.id === id) || null;
    },
    async city(id) {
      if (config.mode === "live") return _liveGet(`/cities/${id}`);
      const c = await _fetchJson(FIXTURES.cities);
      return (c.cities || []).find(x => x.id === id) || null;
    },
    async cityTiles(id) {
      if (config.mode === "live") return _liveGet(`/cities/${id}/tiles`);
      const ct = await _fetchJson(FIXTURES["city-tiles"]);
      return (ct.cities || []).find(x => x.id === id) || null;
    },
    async diplomacyCiv(id) {
      if (config.mode === "live") return _liveGet(`/diplomacy/civs/${id}`);
      const d = await _fetchJson(FIXTURES.diplomacy);
      return (d.civs || []).find(c => c.id === id) || null;
    },
    async combatPreview(attacker_id, defender_q, defender_r) {
      if (config.mode === "live") return _liveGet(_argsToPath("combat-preview", [attacker_id, defender_q, defender_r]));
      const a = await _fetchJson(FIXTURES.armies);
      return a.combat_preview || null;
    },

    // writes — no-ops in json mode, real POST/DELETE in live mode.
    unitAction(id, body)          { return _write("POST",   `/units/${id}/action`, body); },
    endTurn()                     { return _write("POST",   `/turn/end`); },
    queueProduction(cityId, body) { return _write("POST",   `/cities/${cityId}/production`, body); },
    cancelProduction(cityId, pos) { return _write("DELETE", `/cities/${cityId}/production/${pos}`); },
    setCitizenFocus(cityId, focus){ return _write("POST",   `/cities/${cityId}/citizens`, { focus }); },
    renameCity(cityId, name)      { return _write("PATCH",  `/cities/${cityId}/rename`, { name }); },
    research(tech_id)             { return _write("POST",   `/tech/research`, { tech_id }); },
    techQueue(tech_id)            { return _write("POST",   `/tech/queue`,    { tech_id }); },
    techQueueRemove(tech_id)      { return _write("DELETE", `/tech/queue/${tech_id}`); },
    civicResearch(civic_id)       { return _write("POST",   `/civics/research`, { civic_id }); },
    civicQueue(civic_id)          { return _write("POST",   `/civics/queue`,    { civic_id }); },
    civicQueueRemove(civic_id)    { return _write("DELETE", `/civics/queue/${civic_id}`); },
    changeGovernment(government_id){ return _write("POST",  `/government/change`, { government_id }); },
    setPolicies(policies)         { return _write("PUT",    `/government/policies`, { policies }); },
    toggleOverlay(id, active)     { return _write("POST",   `/map/overlays/${id}/toggle`, { active }); },
    dismissNotification(id)       { return _write("DELETE", `/notifications/${id}`); },
    dismissAllNotifications()     { return _write("DELETE", `/notifications`); },
    skipQueueItem(id)             { return _write("POST",   `/turn-queue/${id}/skip`); },
    diplomaticAction(civId, body) { return _write("POST",   `/diplomacy/civs/${civId}/action`, body); },
    updateDeal(body)              { return _write("PUT",    `/diplomacy/deal`, body); },
    proposeDeal()                 { return _write("POST",   `/diplomacy/deal/propose`); },
    formArmy(body)                { return _write("POST",   `/armies`, body); },

    // cache control
    invalidate(...keys) {
      if (!keys.length) { _cache.clear(); return; }
      for (const k of keys) _cache.delete(FIXTURES[k] || k);
    },
  };

  window.api = api;
})();
