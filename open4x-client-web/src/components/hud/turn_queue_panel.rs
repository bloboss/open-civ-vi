//! Turn queue panel — bottom-left HUD overlay.
//!
//! Mirrors `Open4X.html` lines 135-149 + `open4x.js::renderTurnQueue`
//! (lines 512-545). Read-only for now — the skip-item POST and the
//! click-to-target navigation land alongside the WebGL camera signal.

use leptos::prelude::*;

use open4x_protocol::v1::web::turn_queue::TurnQueue;

#[component]
pub fn TurnQueuePanel(
    turn_queue: LocalResource<Option<TurnQueue>>,
) -> impl IntoView {
    let counter = move || {
        let wrap = turn_queue.get();
        let Some(wrap) = wrap.as_deref() else { return "—".to_string() };
        let Some(q) = wrap.as_ref() else { return "—".to_string() };
        let total = q.items.len();
        let required = q.items.iter().filter(|i| i.required).count();
        if required > 0 {
            format!("{total} pending · {required} required")
        } else {
            format!("{total} pending")
        }
    };

    view! {
        <aside class="hud-panel turn-queue-panel" id="turn-queue-panel">
            <div class="hud-panel-h">
                <span class="title">
                    "Turn Queue · "
                    <span class="mono"
                          style="color:var(--ink-4); text-transform:none; letter-spacing:0;">
                        {counter}
                    </span>
                </span>
                <div class="actions">
                    <button class="icon-btn" title="Auto-resolve all">
                        <svg width="12" height="12" viewBox="0 0 16 16" fill="none"
                             stroke="currentColor" stroke-width="1.4">
                            <path d="M3 8h10M9 4l4 4-4 4"
                                  stroke-linecap="round" stroke-linejoin="round" />
                        </svg>
                    </button>
                </div>
            </div>
            <div class="tq-list scrolly">
                {move || render_list(turn_queue)}
            </div>
        </aside>
    }
}

fn render_list(turn_queue: LocalResource<Option<TurnQueue>>) -> AnyView {
    let Some(wrap) = turn_queue.get() else {
        return view! { <div class="notif-empty">"…"</div> }.into_any();
    };
    let Some(q) = wrap.as_ref() else {
        return view! { <div class="notif-empty">"–"</div> }.into_any();
    };
    if q.items.is_empty() {
        return view! {
            <div class="notif-empty">"Queue empty · end turn ready"</div>
        }.into_any();
    }
    let items = q.items.iter().enumerate().map(|(idx, item)| {
        let kind_cls = format!(
            "tq-item kind-{}{}",
            item.kind,
            if item.required { " required" } else { "" }
        );
        let skip = item.skip_label.as_ref().map(|lbl| {
            view! { <button class="tq-skip">{lbl.clone()}</button> }.into_any()
        }).unwrap_or_else(|| ().into_any());
        let req_badge = if item.required {
            view! { <span class="req-badge">"required"</span> }.into_any()
        } else {
            ().into_any()
        };
        view! {
            <div class=kind_cls>
                <span class="tq-num">{(idx + 1).to_string()}</span>
                <div style="flex:1; min-width:0;">
                    <div class="tq-title">
                        {item.title.clone()}
                        " "
                        {req_badge}
                    </div>
                    <div class="tq-desc">{item.desc.clone()}</div>
                </div>
                {skip}
            </div>
        }
    }).collect::<Vec<_>>();
    view! { <>{items}</> }.into_any()
}
