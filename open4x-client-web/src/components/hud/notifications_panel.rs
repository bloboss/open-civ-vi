//! Notifications panel — right-edge HUD overlay.
//!
//! Mirrors `Open4X.html` lines 120-132 + `open4x.js::renderNotifications`
//! (lines 469-505). Driven by the page's `Notifications` LocalResource;
//! dismiss / dismiss-all bump the shared refresh tick.

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use open4x_protocol::v1::web::notifications::Notifications;
use open4x_sdk::endpoints as api;
use open4x_sdk::wasm::WasmClient;

#[component]
pub fn NotificationsPanel(
    notifs: LocalResource<Option<Notifications>>,
    token: RwSignal<Option<String>>,
    tick: RwSignal<u64>,
) -> impl IntoView {
    let turn_label = move || {
        notifs.get()
            .as_deref()
            .and_then(|w| w.as_ref().map(|n| format!("T {}", n.turn)))
            .unwrap_or_else(|| "T —".to_string())
    };

    let on_clear = move |_| {
        let tok = token.get();
        spawn_local(async move {
            let Some(tok) = tok else { return };
            let c = WasmClient::new("").with_token(&tok);
            let _ = api::notifications::dismiss_all(&c).await;
            tick.update(|t| *t += 1);
        });
    };

    view! {
        <aside class="hud-panel notifications-panel" id="notifications-panel">
            <div class="hud-panel-h">
                <span class="title">
                    "Notifications · "
                    <span class="mono" style="color:var(--ink-4); text-transform:none; letter-spacing:0;">
                        {turn_label}
                    </span>
                </span>
                <div class="actions">
                    <button class="icon-btn" title="Clear all" on:click=on_clear>
                        <svg width="12" height="12" viewBox="0 0 16 16" fill="none"
                             stroke="currentColor" stroke-width="1.4">
                            <path d="M3 4h10M5 4V3a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v1M6 7v5M10 7v5M4 4l1 9a1 1 0 0 0 1 1h4a1 1 0 0 0 1-1l1-9"
                                  stroke-linecap="round" />
                        </svg>
                    </button>
                </div>
            </div>
            <div class="notif-list scrolly">
                {move || render_list(notifs, token, tick)}
            </div>
        </aside>
    }
}

fn render_list(
    notifs: LocalResource<Option<Notifications>>,
    token: RwSignal<Option<String>>,
    tick: RwSignal<u64>,
) -> AnyView {
    let Some(wrap) = notifs.get() else {
        return view! { <div class="notif-empty">"…"</div> }.into_any();
    };
    let Some(n) = wrap.as_ref() else {
        return view! { <div class="notif-empty">"–"</div> }.into_any();
    };
    if n.notifications.is_empty() {
        return view! {
            <div class="notif-empty">"All clear · no pending notifications"</div>
        }.into_any();
    }
    let items = n.notifications.iter().map(|item| {
        let id = item.id.clone();
        let kind_cls = match item.kind.as_str() {
            "good" | "warn" | "bad" | "accent" => format!("notif {}", item.kind),
            _ => "notif".to_string(),
        };
        let glyph = category_glyph(&item.category);
        let on_dismiss = move |ev: leptos::ev::MouseEvent| {
            ev.stop_propagation();
            let tok = token.get();
            let id = id.clone();
            spawn_local(async move {
                let Some(tok) = tok else { return };
                let c = WasmClient::new("").with_token(&tok);
                let _ = api::notifications::dismiss(&c, &id).await;
                tick.update(|t| *t += 1);
            });
        };
        view! {
            <div class=kind_cls>
                <span class="notif-icon">{glyph}</span>
                <div style="flex:1; min-width:0;">
                    <div class="notif-title">
                        {item.title.clone()}
                        <span class="notif-cat">{item.category.clone()}</span>
                    </div>
                    <div class="notif-desc">{item.desc.clone()}</div>
                </div>
                <button class="icon-btn" title="Dismiss"
                        style="width:20px; height:20px; flex-shrink:0;"
                        on:click=on_dismiss>
                    <svg width="10" height="10" viewBox="0 0 16 16" fill="none"
                         stroke="currentColor" stroke-width="1.6">
                        <path d="M4 4 L12 12 M12 4 L4 12" stroke-linecap="round" />
                    </svg>
                </button>
            </div>
        }
    }).collect::<Vec<_>>();
    view! { <>{items}</> }.into_any()
}

/// Single-character glyph per notification category — mirrors the
/// switch table in `open4x.js::renderNotifications` (line 478).
fn category_glyph(category: &str) -> &'static str {
    match category {
        "research"    => "\u{1F52C}",  // 🔬
        "environment" => "\u{2248}",   // ≈
        "military"    => "\u{2694}",   // ⚔
        "builder"     => "\u{2713}",   // ✓
        "diplomacy"   => "\u{25CA}",   // ◊
        "civic"       => "\u{25A6}",   // ▦
        "city"        => "\u{25A0}",   // ■
        "production"  => "\u{25A5}",   // ▥
        "religion"    => "\u{2726}",   // ✦
        "economy"     => "$",
        "era"         => "\u{0394}",   // Δ
        _             => "\u{2022}",   // •
    }
}
