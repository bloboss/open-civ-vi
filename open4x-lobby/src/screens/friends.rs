//! Friends screen — Phase 5 polish.
//!
//! Wires the design's Friends tab to the live `/api/v1/friends`
//! surface: list (split into Friends + Requests), Add-friend by
//! `0xAAAA·BBBB·CCCC·DDDD` (the only identity form supported in
//! v1 — email / handle resolution lands with the search route).

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::components::api::friends as friends_api;
use crate::components::{Btn, Panel};

#[component]
pub fn Friends() -> impl IntoView {
    let tick = RwSignal::new(0u32);
    // `None` (after load) means the fetch failed — distinct from a
    // successful empty list, so we don't show "No friends yet" when the
    // server is actually unreachable.
    let rows: LocalResource<Option<Vec<friends_api::FriendView>>> = LocalResource::new(move || {
        // Re-fetch on every tick increment.
        let _ = tick.get();
        async move { friends_api::list().await.ok() }
    });

    let input = RwSignal::new(String::new());
    let pending = RwSignal::new(false);
    let err = RwSignal::new(String::new());

    // True if the input parses as a 16-hex PlayerId (canonical
    // dot-grouped form or bare hex). Skips the search hop.
    fn looks_like_hex_pid(s: &str) -> bool {
        let body = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
        let stripped: String = body.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        stripped.len() == 16
    }

    let on_add = move |_| {
        let q = input.get_untracked().trim().to_string();
        if q.is_empty() {
            err.set("paste a player ID, email, or handle first".into());
            return;
        }
        err.set(String::new());
        pending.set(true);
        spawn_local(async move {
            // Resolve to a PlayerId. If the user pasted a hex form
            // we skip the round-trip; otherwise the lobby's search
            // route maps email/handle/OpenID → PlayerId (gated by
            // the target's `discoverable_by_id` preference).
            let pid_to_request = if looks_like_hex_pid(&q) {
                Some(q.clone())
            } else {
                match friends_api::search(q.clone()).await {
                    Ok(matches) if matches.is_empty() => {
                        err.set(format!("no match for {q}"));
                        None
                    }
                    Ok(matches) if matches.len() == 1 => {
                        Some(matches[0].player_id.clone())
                    }
                    Ok(matches) => {
                        err.set(format!(
                            "multiple matches ({}). paste the player ID directly to disambiguate.",
                            matches.len()
                        ));
                        None
                    }
                    Err(e) => {
                        err.set(format!("search failed: {e}"));
                        None
                    }
                }
            };
            if let Some(pid) = pid_to_request {
                match friends_api::request(pid).await {
                    Ok(()) => {
                        input.set(String::new());
                        tick.update(|t| *t += 1);
                    }
                    Err(e) => err.set(e.to_string()),
                }
            }
            pending.set(false);
        });
    };

    view! {
        <div style="flex:1; overflow:auto">
            <div class="content-header">
                <div class="title">"Friends"</div>
                <span class="crumbs">
                    "// search by player ID, email, atproto handle, or OpenID URL"
                </span>
                <div class="actions">
                    <Btn
                        variant="accent"
                        disabled=Signal::derive(move || pending.get())
                        on_click=Callback::new(on_add)
                    >
                        {move || if pending.get() { "Sending…" } else { "+ Add friend" }}
                    </Btn>
                </div>
            </div>

            <Panel>
                <div class="filter-bar" style="margin:0">
                    <span class="muted xsmall" style="padding-left:4px">"⌕"</span>
                    <input
                        class="filter-search"
                        placeholder="alice@example.com  ·  did:plc:…  ·  0xA9C3·7F12·EE04·AB55"
                        prop:value=move || input.get()
                        on:input=move |ev| {
                            use wasm_bindgen::JsCast as _;
                            if let Some(el) = ev.target()
                                .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                            {
                                input.set(el.value());
                            }
                        }
                    />
                </div>
                {move || (!err.get().is_empty()).then(|| view! {
                    <p class="xsmall" style="color:var(--accent); margin:8px 0 0">
                        {err.get()}
                    </p>
                })}
            </Panel>

            <Suspense fallback=move || view! { <p class="muted xsmall">"Loading…"</p> }>
                {move || rows.get().map(|wrap| {
                    let Some(all) = (*wrap).clone() else {
                        return view! {
                            <p class="muted xsmall">"Couldn't load friends — try refreshing."</p>
                        }.into_any();
                    };
                    let accepted: Vec<_> = all.iter()
                        .filter(|r| r.status == "accepted")
                        .cloned()
                        .collect();
                    let incoming: Vec<_> = all.iter()
                        .filter(|r| r.status == "pending_incoming")
                        .cloned()
                        .collect();
                    let outgoing: Vec<_> = all.iter()
                        .filter(|r| r.status == "pending_outgoing")
                        .cloned()
                        .collect();
                    view! {
                        <Panel>
                            <div class="h3" style="margin-bottom:10px">{
                                format!("Friends ({})", accepted.len())
                            }</div>
                            {if accepted.is_empty() {
                                view! { <p class="muted small">"No friends yet."</p> }.into_any()
                            } else {
                                view! {
                                    <div class="col" style="gap:6px">
                                        {accepted.into_iter().map(|f| {
                                            let pid = f.player_id.clone();
                                            let pid_for_btn = pid.clone();
                                            view! {
                                                <div class="row between center-y">
                                                    <code style="font-family:var(--font-mono); font-size:12px">
                                                        {pid}
                                                    </code>
                                                    <Btn
                                                        variant="bare"
                                                        size="sm"
                                                        on_click=Callback::new(move |_| {
                                                            let id = pid_for_btn.clone();
                                                            spawn_local(async move {
                                                                let _ = friends_api::unfriend(&id).await;
                                                                tick.update(|t| *t += 1);
                                                            });
                                                        })
                                                    >"unfriend"</Btn>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_any()
                            }}
                        </Panel>

                        <Panel>
                            <div class="h3" style="margin-bottom:10px">{
                                format!("Requests · {} incoming · {} outgoing",
                                    incoming.len(), outgoing.len())
                            }</div>
                            {if incoming.is_empty() && outgoing.is_empty() {
                                view! { <p class="muted small">"No pending requests."</p> }.into_any()
                            } else {
                                view! {
                                    <div class="col" style="gap:6px">
                                        {incoming.into_iter().map(|f| {
                                            let pid = f.player_id.clone();
                                            let pid_accept = pid.clone();
                                            let pid_decline = pid.clone();
                                            view! {
                                                <div class="row between center-y">
                                                    <span>
                                                        <code style="font-family:var(--font-mono); font-size:12px">{pid}</code>
                                                        " "
                                                        <span class="muted xsmall">"wants to add you"</span>
                                                    </span>
                                                    <span class="row gap-xs">
                                                        <Btn
                                                            variant="accent"
                                                            size="sm"
                                                            on_click=Callback::new(move |_| {
                                                                let id = pid_accept.clone();
                                                                spawn_local(async move {
                                                                    let _ = friends_api::accept(&id).await;
                                                                    tick.update(|t| *t += 1);
                                                                });
                                                            })
                                                        >"accept"</Btn>
                                                        <Btn
                                                            variant="bare"
                                                            size="sm"
                                                            on_click=Callback::new(move |_| {
                                                                let id = pid_decline.clone();
                                                                spawn_local(async move {
                                                                    let _ = friends_api::unfriend(&id).await;
                                                                    tick.update(|t| *t += 1);
                                                                });
                                                            })
                                                        >"decline"</Btn>
                                                    </span>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                        {outgoing.into_iter().map(|f| {
                                            let pid = f.player_id.clone();
                                            let pid_cancel = pid.clone();
                                            view! {
                                                <div class="row between center-y">
                                                    <span>
                                                        <code style="font-family:var(--font-mono); font-size:12px">{pid}</code>
                                                        " "
                                                        <span class="muted xsmall">"awaiting their response"</span>
                                                    </span>
                                                    <Btn
                                                        variant="bare"
                                                        size="sm"
                                                        on_click=Callback::new(move |_| {
                                                            let id = pid_cancel.clone();
                                                            spawn_local(async move {
                                                                let _ = friends_api::unfriend(&id).await;
                                                                tick.update(|t| *t += 1);
                                                            });
                                                        })
                                                    >"cancel"</Btn>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_any()
                            }}
                        </Panel>
                    }.into_any()
                })}
            </Suspense>
        </div>
    }
}
