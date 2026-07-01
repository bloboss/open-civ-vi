//! Profile screen — wired to `GET /api/v1/me` and `PATCH /api/v1/me`.
//!
//! On mount fetches the authenticated profile via
//! `components::api::me::get` and seeds the form fields. Each field /
//! preference is a local `RwSignal`; the "Save profile" button posts
//! the diff back through `me::patch`. Linked-identities list reads
//! straight from `MeView.identities` and `+ link another` / `unlink`
//! buttons are still TODO behind their respective HTTP routes.

use std::sync::Arc;

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::components::api::me as me_api;
use crate::components::qr::qr_svg;
use crate::components::{
    Btn, Panel, Popup, PopupBody, PopupSize, PopupTrigger, Segmented, Toggle, segmented::Segment,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum SaveState {
    #[default]
    Idle,
    Saving,
    Saved,
    Error(String),
}

#[component]
pub fn Profile(#[prop(optional)] on_signout: Option<Callback<()>>) -> impl IntoView {
    let me_tick = RwSignal::new(0u32);
    let me = LocalResource::new(move || {
        let _ = me_tick.get();
        async { me_api::get().await.ok() }
    });

    let preferred_name = RwSignal::new(String::new());
    let pronouns = RwSignal::new(String::new());
    let bio = RwSignal::new(String::new());
    let density = RwSignal::new("comfortable".to_string());
    let scheme = RwSignal::new("paper".to_string());
    let kbd = RwSignal::new(true);
    let notifs = RwSignal::new(true);
    let discoverable = RwSignal::new(true);
    let player_id_text = RwSignal::new(String::from("—"));
    let identities = RwSignal::new(Vec::<me_api::IdentityView>::new());
    let save_state = RwSignal::new(SaveState::Idle);
    let avatar_url: RwSignal<Option<String>> = RwSignal::new(None);
    let avatar_pending = RwSignal::new(false);
    let avatar_error = RwSignal::new(String::new());

    // Seed signals once the resource arrives.
    Effect::new(move |_| {
        let Some(wrap) = me.get() else { return };
        let Some(view) = (*wrap).clone() else { return };
        preferred_name.set(view.preferred_name);
        pronouns.set(view.pronouns);
        bio.set(view.bio);
        density.set(view.prefs.density);
        scheme.set(view.prefs.color_scheme);
        kbd.set(view.prefs.keyboard_nav);
        notifs.set(view.prefs.turn_notifications);
        discoverable.set(view.prefs.discoverable_by_id);
        player_id_text.set(view.player_id);
        identities.set(view.identities);
        avatar_url.set(view.avatar_url);
    });

    let density_opts = Signal::derive(|| {
        ["compact", "comfortable", "spacious"]
            .iter()
            .map(|s| Segment::from_str(s))
            .collect()
    });
    let scheme_opts = Signal::derive(|| {
        ["paper", "ink", "auto"]
            .iter()
            .map(|s| Segment::from_str(s))
            .collect()
    });

    let on_save = move |_| {
        save_state.set(SaveState::Saving);
        let body = me_api::PatchMeBody {
            preferred_name: Some(preferred_name.get_untracked()),
            pronouns: Some(pronouns.get_untracked()),
            bio: Some(bio.get_untracked()),
            prefs: Some(me_api::Preferences {
                density: density.get_untracked(),
                color_scheme: scheme.get_untracked(),
                keyboard_nav: kbd.get_untracked(),
                turn_notifications: notifs.get_untracked(),
                discoverable_by_id: discoverable.get_untracked(),
            }),
        };
        spawn_local(async move {
            match me_api::patch(body).await {
                Ok(_) => save_state.set(SaveState::Saved),
                Err(e) => save_state.set(SaveState::Error(e.to_string())),
            }
        });
    };

    let saving = Signal::derive(move || matches!(save_state.get(), SaveState::Saving));

    view! {
        <div style="flex:1; overflow:auto">
            <div class="content-header">
                <div class="title">"Profile & settings"</div>
                <span class="crumbs">
                    "player_id: "
                    <code class="trigger" style="font-family:var(--font-mono)"
                          title="Your unique identifier on the platform.">
                        {move || player_id_text.get()}
                    </code>
                </span>
            </div>

            <div class="profile-grid">
                <Panel>
                    <div class="col" style="align-items:center; gap:12px">
                        <div class="avatar">
                            {move || match avatar_url.get() {
                                // Cache-bust on every refresh by appending the
                                // current `avatar_pending` tick — when the user
                                // re-uploads, we toggle the flag, which forces
                                // the browser to re-fetch.
                                Some(u) if !u.is_empty() => view! {
                                    <img
                                        src=u
                                        alt="avatar"
                                        style="width:100%; height:100%; border-radius:50%; object-fit:cover"
                                    />
                                }.into_any(),
                                _ => view! {
                                    <span>{
                                        preferred_name.get()
                                            .chars()
                                            .next()
                                            .map(|c| c.to_string())
                                            .unwrap_or_else(|| "?".into())
                                    }</span>
                                }.into_any(),
                            }}
                        </div>
                        // Hidden file input + visible "change" Btn that
                        // triggers .click() on the input. The file's first
                        // entry is uploaded via me_api::upload_avatar.
                        <input
                            id="avatar-file"
                            r#type="file"
                            accept="image/png,image/jpeg"
                            style="display:none"
                            on:change=move |ev| {
                                use wasm_bindgen::JsCast as _;
                                let file = ev.target()
                                    .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                    .and_then(|el| el.files())
                                    .and_then(|fl| fl.item(0));
                                let Some(file) = file else { return; };
                                avatar_pending.set(true);
                                avatar_error.set(String::new());
                                spawn_local(async move {
                                    match me_api::upload_avatar(file).await {
                                        Ok(url) => {
                                            // Append a cache-buster so browsers
                                            // pick up the new bytes; same path
                                            // otherwise.
                                            let now = js_sys::Date::now() as u64;
                                            let busted = format!("{url}?t={now}");
                                            avatar_url.set(Some(busted));
                                        }
                                        Err(e) => avatar_error.set(e.to_string()),
                                    }
                                    avatar_pending.set(false);
                                });
                            }
                        />
                        <Btn
                            variant="ghost"
                            size="sm"
                            disabled=Signal::derive(move || avatar_pending.get())
                            on_click=Callback::new(move |_| {
                                use wasm_bindgen::JsCast as _;
                                if let Some(input) = web_sys::window()
                                    .and_then(|w| w.document())
                                    .and_then(|d| d.get_element_by_id("avatar-file"))
                                    .and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().ok())
                                {
                                    input.click();
                                }
                            })
                        >
                            {move || if avatar_pending.get() { "Uploading…" } else { "change" }}
                        </Btn>
                        {move || (!avatar_error.get().is_empty()).then(|| view! {
                            <p class="xsmall" style="color:var(--accent); margin:0">
                                {avatar_error.get()}
                            </p>
                        })}
                    </div>
                    <hr class="divider" />
                    <div class="h3" style="margin-bottom:10px">"Quick actions"</div>
                    <div class="col" style="gap:4px">
                        <Btn
                            variant="bare"
                            size="sm"
                            on_click=Callback::new(move |_| {
                                let id = player_id_text.get_untracked();
                                if let Some(win) = web_sys::window() {
                                    let _ = win.navigator().clipboard().write_text(&id);
                                }
                            })
                        >"⎘ Copy player ID"</Btn>
                        {
                            let pid_for_qr = player_id_text;
                            let qr_content = Arc::new(move || {
                                let pid = pid_for_qr.get();
                                let svg = qr_svg(&pid, 220);
                                view! {
                                    <PopupBody>
                                        <p class="xsmall muted" style="text-align:center; margin-bottom:6px">
                                            "Scan to add as a friend"
                                        </p>
                                        <div
                                            style="display:flex; justify-content:center; padding:8px"
                                            inner_html=svg
                                        ></div>
                                        <p class="xsmall" style="text-align:center; font-family:var(--font-mono); margin-top:4px">
                                            {pid}
                                        </p>
                                    </PopupBody>
                                }.into_any()
                            });
                            view! {
                                <Popup
                                    title="Invite QR"
                                    size=PopupSize::Narrow
                                    trigger=PopupTrigger::Click
                                    content=qr_content
                                >
                                    <Btn variant="bare" size="sm">"▦ Show invite QR"</Btn>
                                </Popup>
                            }
                        }
                        <Btn variant="bare" size="sm">"↓ Export save data"</Btn>
                        <Btn
                            variant="bare"
                            size="sm"
                            on_click=Callback::new(move |_| {
                                if let Some(cb) = on_signout {
                                    cb.run(());
                                }
                            })
                        >"→ Sign out"</Btn>
                    </div>
                </Panel>

                <div class="col">
                    <Panel>
                        <div class="row between center-y" style="margin-bottom:14px">
                            <div class="h3">"Profile"</div>
                            <div class="row gap-sm center-y">
                                {move || match save_state.get() {
                                    SaveState::Saved => view! {
                                        <span class="xsmall" style="color:var(--good)">"saved ✓"</span>
                                    }.into_any(),
                                    SaveState::Error(msg) => view! {
                                        <span class="xsmall" style="color:var(--accent)">{msg}</span>
                                    }.into_any(),
                                    _ => view! { <span /> }.into_any(),
                                }}
                                <Btn
                                    variant="primary"
                                    size="sm"
                                    disabled=saving
                                    on_click=Callback::new(on_save)
                                >
                                    {move || if saving.get() { "saving…" } else { "save" }}
                                </Btn>
                            </div>
                        </div>
                        <div class="field">
                            <label>"Preferred name"</label>
                            <input
                                class="input"
                                prop:value=move || preferred_name.get()
                                on:input=move |ev| {
                                    use wasm_bindgen::JsCast as _;
                                    if let Some(el) = ev.target()
                                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                    {
                                        preferred_name.set(el.value());
                                    }
                                }
                            />
                            <span class="hint">"Shown to other players in invites & chat."</span>
                        </div>
                        <div class="field">
                            <label>"Pronouns (optional)"</label>
                            <input
                                class="input"
                                prop:value=move || pronouns.get()
                                on:input=move |ev| {
                                    use wasm_bindgen::JsCast as _;
                                    if let Some(el) = ev.target()
                                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                    {
                                        pronouns.set(el.value());
                                    }
                                }
                            />
                        </div>
                        <div class="field">
                            <label>"Bio"</label>
                            <textarea
                                class="input"
                                rows="2"
                                prop:value=move || bio.get()
                                on:input=move |ev| {
                                    use wasm_bindgen::JsCast as _;
                                    if let Some(el) = ev.target()
                                        .and_then(|t| t.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
                                    {
                                        bio.set(el.value());
                                    }
                                }
                            ></textarea>
                            <span class="hint">"Markdown supported. Visible on invite cards."</span>
                        </div>
                    </Panel>

                    <Panel>
                        <div class="row between center-y" style="margin-bottom:12px">
                            <div class="h3">"Linked identities"</div>
                            <Btn variant="ghost" size="sm">"+ link another"</Btn>
                        </div>

                        <Suspense fallback=move || view! {
                            <p class="muted xsmall">"Loading…"</p>
                        }>
                            {move || identities.get().is_empty().then(|| view! {
                                <p class="muted xsmall">"No identities linked yet."</p>
                            })}
                            {move || {
                                let total = identities.get().len();
                                identities.get().into_iter().enumerate().map(|(i, id)| {
                                    let is_primary_email = id.kind == "email"
                                        && id.primary.unwrap_or(false);
                                    let kind_label = match id.kind.as_str() {
                                        "email"   => if is_primary_email { "EMAIL · primary".into() } else { "EMAIL".into() },
                                        "oidc"    => "OPENID".into(),
                                        "atproto" => "ATPROTO".into(),
                                        other     => other.to_uppercase(),
                                    };
                                    let row_class = if is_primary_email { "id-row primary" } else { "id-row" };
                                    let row_id = id.id.clone();
                                    // Refuse to show a working unlink when this would orphan
                                    // the account; the server returns the same error but the
                                    // client UX is clearer this way.
                                    let last_one = total <= 1;
                                    let row_label = id.label.clone();
                                    let action_btn = if is_primary_email {
                                        view! { <Btn variant="bare" size="sm">"manage"</Btn> }.into_any()
                                    } else {
                                        let confirm_content = {
                                            let row_id = row_id.clone();
                                            let row_label = row_label.clone();
                                            Arc::new(move || {
                                                let row_id = row_id.clone();
                                                let row_label = row_label.clone();
                                                view! {
                                                    <PopupBody>
                                                        <p>"Unlink "
                                                            <code>{row_label.clone()}</code>
                                                            "? You'll need at least one other "
                                                            "identity linked to keep signing in."
                                                        </p>
                                                    </PopupBody>
                                                    <crate::components::PopupActions right=true>
                                                        <Btn
                                                            variant="bare"
                                                            size="sm"
                                                            on_click=Callback::new(move |_| {
                                                                let id = row_id.clone();
                                                                spawn_local(async move {
                                                                    let _ = me_api::unlink_identity(&id).await;
                                                                    me_tick.update(|t| *t += 1);
                                                                });
                                                            })
                                                        >"yes, unlink"</Btn>
                                                    </crate::components::PopupActions>
                                                }
                                                .into_any()
                                            })
                                        };
                                        if last_one {
                                            view! {
                                                <Btn variant="bare" size="sm" disabled=Signal::derive(|| true)>
                                                    "unlink"
                                                </Btn>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <Popup
                                                    title="Confirm unlink"
                                                    size=PopupSize::Narrow
                                                    trigger=PopupTrigger::Click
                                                    content=confirm_content
                                                >
                                                    <Btn variant="bare" size="sm">"unlink"</Btn>
                                                </Popup>
                                            }.into_any()
                                        }
                                    };
                                    // Verify-email CTA: shown on email rows that
                                    // are not yet verified. Clicking mints + mails
                                    // a fresh magic link via the verify-start
                                    // route; the user clicks the email link, the
                                    // server flips `verified=true`, the list
                                    // refreshes and this button disappears.
                                    let is_unverified_email = id.kind == "email"
                                        && !id.verified.unwrap_or(false);
                                    let verify_btn = if is_unverified_email {
                                        let row_id = row_id.clone();
                                        view! {
                                            <Btn
                                                variant="ghost"
                                                size="sm"
                                                on_click=Callback::new(move |_| {
                                                    let id = row_id.clone();
                                                    spawn_local(async move {
                                                        let _ = me_api::start_verify_identity(&id).await;
                                                    });
                                                })
                                            >"verify"</Btn>
                                        }.into_any()
                                    } else {
                                        view! { <span></span> }.into_any()
                                    };
                                    view! {
                                        <div class=row_class data-i=i>
                                            <span class="id-type">{kind_label}</span>
                                            <span class="id-val">{id.label.clone()}</span>
                                            {verify_btn}
                                            {action_btn}
                                        </div>
                                    }
                                }).collect::<Vec<_>>()
                            }}
                        </Suspense>

                        <p class="muted xsmall" style="margin-top:10px">
                            "All identities map to the same player. "
                            "Friends can find you by any of them."
                        </p>
                    </Panel>

                    <Panel>
                        <div class="h3" style="margin-bottom:12px">"Preferences"</div>

                        <div class="param-row">
                            <div class="label">"Density"</div>
                            <div class="control"><Segmented options=density_opts value=density /></div>
                            <div class="value muted xsmall">"→ tweaks panel"</div>
                        </div>

                        <div class="param-row">
                            <div class="label">"Color scheme"</div>
                            <div class="control"><Segmented options=scheme_opts value=scheme /></div>
                            <div class="value muted xsmall">{move || scheme.get()}</div>
                        </div>

                        <div class="param-row">
                            <div class="label">"Keyboard nav"</div>
                            <div class="control">
                                <Toggle on=kbd on_change=Callback::new(move |v| kbd.set(v)) />
                            </div>
                            <div class="value muted xsmall">"vim bindings"</div>
                        </div>

                        <div class="param-row">
                            <div class="label">"Turn notifications"</div>
                            <div class="control">
                                <Toggle on=notifs on_change=Callback::new(move |v| notifs.set(v)) />
                            </div>
                            <div class="value muted xsmall">"email + push"</div>
                        </div>

                        <div class="param-row">
                            <div class="label">"Discoverable by ID"</div>
                            <div class="control">
                                <Toggle on=discoverable on_change=Callback::new(move |v| discoverable.set(v)) />
                            </div>
                            <div class="value muted xsmall">"others can invite"</div>
                        </div>
                    </Panel>
                </div>
            </div>
        </div>
    }
}
