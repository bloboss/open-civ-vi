//! Real-game tile thumbnails (Phase 5 polish).
//!
//! The browser fetches `/api/v1/games/{id}/thumbnail` (same-origin,
//! cookie auth). The lobby's handler issues a Bearer-authed
//! request against `<server_url>/api/v1/world/snapshot` server-
//! side, reduces the response to a `(q, r, terrain)` triple grid,
//! and returns it as JSON. The SPA caches the response in an
//! in-session `RwSignal<HashMap<game_id, ThumbnailGrid>>` so
//! repeat mounts (filter changes, sort flips) don't re-fetch.

#![cfg(feature = "csr")]

use std::collections::HashMap;

use leptos::prelude::*;
use serde::Deserialize;

use super::api::http::fetch_json;

/// One tile reduced to what the minimap needs.
#[derive(Debug, Clone)]
pub struct ThumbCell {
    pub q: i32,
    pub r: i32,
    pub terrain: String,
    pub fog: bool,
    /// Three-state ownership: `Some(true)` mine, `Some(false)`
    /// foreign, `None` unowned.
    pub owned_by_me: Option<bool>,
    pub city: bool,
    pub capital: bool,
}

/// Reduced shape: world dimensions + the tile cells. Cached in
/// the [`ThumbnailCache`] under the per-game `server_token`.
#[derive(Debug, Clone)]
pub struct ThumbnailGrid {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<ThumbCell>,
}

/// Per-token state in the cache. `Pending` covers the in-flight
/// case so repeat mounts during a fetch don't all kick off
/// duplicate requests.
#[derive(Clone)]
pub enum ThumbnailEntry {
    Pending,
    Ready(ThumbnailGrid),
    Failed(String),
}

#[derive(Clone)]
pub struct ThumbnailCache {
    inner: RwSignal<HashMap<String, ThumbnailEntry>, LocalStorage>,
}

impl ThumbnailCache {
    pub fn new() -> Self {
        Self {
            inner: RwSignal::new_local(HashMap::new()),
        }
    }

    /// Snapshot read; cheap-ish (Hash clone). The component only
    /// re-renders when the signal is `.get()`-ed in a reactive
    /// scope, so callers should call this from inside a
    /// `move ||` closure.
    pub fn get_entry(&self, token: &str) -> Option<ThumbnailEntry> {
        self.inner.with(|m| m.get(token).cloned())
    }

    pub fn set_entry(&self, token: String, entry: ThumbnailEntry) {
        self.inner.update(|m| {
            m.insert(token, entry);
        });
    }
}

impl Default for ThumbnailCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Mount once at the App root. Subsequent `expect_context` calls
/// in any descendant return the same shared cache.
pub fn provide_thumbnail_cache() {
    provide_context(ThumbnailCache::new());
}

pub fn use_thumbnail_cache() -> ThumbnailCache {
    expect_context::<ThumbnailCache>()
}

/// Wire shape returned by `GET /api/v1/games/{id}/thumbnail`.
/// Mirrors [`ThumbnailGrid`] for direct deserialization.
#[derive(Deserialize)]
struct WireThumb {
    width: u32,
    height: u32,
    #[serde(default)]
    cells: Vec<WireCell>,
}

#[derive(Deserialize)]
struct WireCell {
    q: i32,
    r: i32,
    terrain: String,
    #[serde(default)]
    fog: bool,
    #[serde(default)]
    owned_by_me: Option<bool>,
    #[serde(default)]
    city: bool,
    #[serde(default)]
    capital: bool,
}

/// Same-origin fetch via the lobby's thumbnail proxy. Cookie auth
/// (the lobby's session) replaces the cross-origin Bearer hand-
/// shake. The proxy does the bearer'd hop to the game server
/// server-side using the games row's stored `server_token`.
pub async fn fetch_thumbnail(game_id: &str) -> Result<ThumbnailGrid, String> {
    let url = format!("/api/v1/games/{game_id}/thumbnail");
    let wire: WireThumb = fetch_json::<WireThumb, ()>("GET", &url, None)
        .await
        .map_err(|e| e.to_string())?;
    let cells = wire
        .cells
        .into_iter()
        .map(|c| ThumbCell {
            q: c.q,
            r: c.r,
            terrain: c.terrain,
            fog: c.fog,
            owned_by_me: c.owned_by_me,
            city: c.city,
            capital: c.capital,
        })
        .collect();
    Ok(ThumbnailGrid {
        width: wire.width,
        height: wire.height,
        cells,
    })
}

/// Kick off (or no-op if already cached) a background fetch for
/// `game_id`. Idempotent: pending or ready entries are left alone.
/// Returns immediately; the cache signal updates when the fetch
/// resolves and any subscribed view re-renders.
pub fn ensure_fetched(cache: &ThumbnailCache, game_id: String) {
    if game_id.is_empty() {
        return;
    }
    if cache.get_entry(&game_id).is_some() {
        return;
    }
    cache.set_entry(game_id.clone(), ThumbnailEntry::Pending);
    let cache_clone: ThumbnailCache = cache.clone();
    leptos::task::spawn_local(async move {
        match fetch_thumbnail(&game_id).await {
            Ok(grid) => cache_clone.set_entry(game_id, ThumbnailEntry::Ready(grid)),
            Err(e) => cache_clone.set_entry(game_id, ThumbnailEntry::Failed(e)),
        }
    });
}

/// Combined classes for a single cell. Layers four signals:
///   1. Water terrain → `water` (no ownership tinting; oceans are
///      neutral by definition).
///   2. City centre → `city` or `city-capital` (capital wins).
///   3. Ownership → `land-self` (mine), `land-foreign` (other),
///      `land` (unowned).
///   4. Fog → adds `fog` modifier so the SPA can dim it.
///
/// All four are mapped to existing CSS classes used by the
/// design's `.svg-map` rules; the new `.fog` / `.land-foreign` /
/// `.city` / `.city-capital` classes are absorbed by the SPA's
/// stylesheet so the thumbnail blends with the rest of the
/// landing chrome.
pub fn cell_class(cell: &ThumbCell) -> String {
    let t = cell.terrain.to_lowercase();
    if t.contains("ocean")
        || t.contains("coast")
        || t.contains("water")
        || t.contains("lake")
        || t.contains("reef")
    {
        let mut cls = "water".to_string();
        if cell.fog {
            cls.push_str(" fog");
        }
        return cls;
    }
    if cell.city {
        let base = if cell.capital { "city-capital" } else { "city" };
        return if cell.fog {
            format!("{base} fog")
        } else {
            base.into()
        };
    }
    let owner = match cell.owned_by_me {
        Some(true) => "land-self",
        Some(false) => "land-foreign",
        None => "land",
    };
    if cell.fog {
        format!("{owner} fog")
    } else {
        owner.into()
    }
}

/// Backwards-compat: terrain-only mapping. Prefer
/// [`cell_class`] for the full-context render path. Kept so the
/// MiniMap fallback (no `cells` prop) and the existing call
/// sites still link.
pub fn terrain_class(terrain: &str) -> &'static str {
    let t = terrain.to_lowercase();
    if t.contains("ocean")
        || t.contains("coast")
        || t.contains("water")
        || t.contains("lake")
        || t.contains("reef")
    {
        "water"
    } else if t.contains("hill") || t.contains("mountain") {
        "land-self"
    } else {
        "land"
    }
}
