//! Presets screen — Phase 5 polish.
//!
//! Wires the design's Presets tab to `/api/v1/presets`:
//! - "Built-in" panel renders `newgame::builtin_presets()`; each
//!   row's "load" pushes the config into the wizard via `on_load`.
//! - "My presets" lists the user's saved rows, each with Load + Delete.
//! - "↑ import JSON…" toggles a textarea where the user can paste
//!   a name + a JSON body, then Save persists it.
//!
//! Saving a config lives in the New-game wizard ("+ save preset" in
//! the footer); both built-in and saved rows here load back into that
//! wizard via the `on_load` callback.

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::components::api::presets as presets_api;
use crate::components::{Btn, Panel};

#[component]
pub fn Presets(
    /// Load a saved preset's `body_json` into the New-game wizard.
    #[prop(optional)]
    on_load: Option<Callback<String>>,
) -> impl IntoView {
    let tick = RwSignal::new(0u32);
    // `None` (after load) = the fetch failed, kept distinct from a
    // successful empty list so a server error doesn't masquerade as
    // "no saved presets yet".
    let rows: LocalResource<Option<Vec<presets_api::PresetView>>> = LocalResource::new(move || {
        let _ = tick.get();
        async move { presets_api::list().await.ok() }
    });

    let show_import = RwSignal::new(false);
    let name = RwSignal::new(String::new());
    let body = RwSignal::new(String::new());
    let pending = RwSignal::new(false);
    let err = RwSignal::new(String::new());

    let on_save = move |_| {
        let n = name.get_untracked().trim().to_string();
        let b = body.get_untracked();
        if n.is_empty() {
            err.set("name required".into());
            return;
        }
        if b.trim().is_empty() {
            err.set("paste a JSON body".into());
            return;
        }
        err.set(String::new());
        pending.set(true);
        spawn_local(async move {
            match presets_api::create(n, b).await {
                Ok(_) => {
                    name.set(String::new());
                    body.set(String::new());
                    show_import.set(false);
                    tick.update(|t| *t += 1);
                }
                Err(e) => err.set(e.to_string()),
            }
            pending.set(false);
        });
    };

    view! {
        <div style="flex:1; overflow:auto">
            <div class="content-header">
                <div class="title">"Presets"</div>
                <span class="crumbs">"// save / load / import wizard configs"</span>
                <div class="actions">
                    <Btn
                        variant="ghost"
                        size="sm"
                        on_click=Callback::new(move |_| {
                            show_import.update(|v| *v = !*v);
                        })
                    >
                        {move || if show_import.get() { "× cancel import" } else { "↑ import JSON…" }}
                    </Btn>
                </div>
            </div>

            {move || show_import.get().then(|| view! {
                <Panel>
                    <div class="h3" style="margin-bottom:10px">"Import preset"</div>
                    <div class="field" style="margin-bottom:8px">
                        <label class="muted xsmall">"Name"</label>
                        <input
                            class="input"
                            placeholder="e.g. Standard prince"
                            prop:value=move || name.get()
                            on:input=move |ev| {
                                use wasm_bindgen::JsCast as _;
                                if let Some(el) = ev.target()
                                    .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                {
                                    name.set(el.value());
                                }
                            }
                        />
                    </div>
                    <div class="field" style="margin-bottom:8px">
                        <label class="muted xsmall">"JSON body"</label>
                        <textarea
                            class="input mono"
                            rows="6"
                            placeholder=r#"{"map":"continents","size":"std","difficulty":"prince",…}"#
                            prop:value=move || body.get()
                            on:input=move |ev| {
                                use wasm_bindgen::JsCast as _;
                                if let Some(el) = ev.target()
                                    .and_then(|t| t.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
                                {
                                    body.set(el.value());
                                }
                            }
                        ></textarea>
                    </div>
                    {move || (!err.get().is_empty()).then(|| view! {
                        <p class="xsmall" style="color:var(--accent); margin:0 0 8px">{err.get()}</p>
                    })}
                    <Btn
                        variant="accent"
                        disabled=Signal::derive(move || pending.get())
                        on_click=Callback::new(on_save)
                    >
                        {move || if pending.get() { "Saving…" } else { "Save preset" }}
                    </Btn>
                </Panel>
            })}

            <Panel>
                <div class="h3" style="margin-bottom:10px">"Built-in"</div>
                <div class="col" style="gap:6px">
                    {let builtins = crate::screens::newgame::builtin_presets();
                     let total = builtins.len();
                     builtins.into_iter().enumerate().map(|(i, b)| {
                        let border = if i + 1 == total { "" } else { "border-bottom:1px solid var(--hairline-2); " };
                        view! {
                            <div class="row between center-y" style=format!("{border}padding:6px 0")>
                                <div>
                                    <div style="font-weight:600">{b.name}</div>
                                    <div class="muted xsmall">{b.desc}</div>
                                </div>
                                {on_load.map(|cb| {
                                    let body = b.body_json.clone();
                                    view! {
                                        <Btn
                                            variant="ghost"
                                            size="sm"
                                            on_click=Callback::new(move |_| cb.run(body.clone()))
                                        >"load"</Btn>
                                    }
                                })}
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </Panel>

            <Suspense fallback=move || view! {
                <Panel>
                    <div class="h3" style="margin-bottom:10px">"My presets"</div>
                    <p class="muted xsmall">"Loading…"</p>
                </Panel>
            }>
                {move || rows.get().map(|wrap| {
                    let Some(mine) = (*wrap).clone() else {
                        return view! {
                            <Panel>
                                <div class="h3" style="margin-bottom:10px">"My presets"</div>
                                <p class="muted xsmall">"Couldn't load presets — try refreshing."</p>
                            </Panel>
                        }.into_any();
                    };
                    view! {
                        <Panel>
                            <div class="h3" style="margin-bottom:10px">
                                {format!("My presets ({})", mine.len())}
                            </div>
                            {if mine.is_empty() {
                                view! {
                                    <p class="muted small">
                                        "No saved presets yet. Click " <strong>"↑ import JSON…"</strong>
                                        " to paste a configuration."
                                    </p>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="col" style="gap:6px">
                                        {mine.into_iter().map(|p| {
                                            let id_for_del = p.id.clone();
                                            let body_for_load = p.body_json.clone();
                                            view! {
                                                <div class="row between center-y" style="border-bottom:1px solid var(--hairline-2); padding:6px 0">
                                                    <div>
                                                        <div style="font-weight:600">{p.name.clone()}</div>
                                                        <div class="muted xsmall" style="font-family:var(--font-mono); white-space:nowrap; overflow:hidden; text-overflow:ellipsis; max-width:520px">
                                                            {p.body_json.clone()}
                                                        </div>
                                                    </div>
                                                    <div class="row" style="gap:4px">
                                                        {on_load.map(|cb| {
                                                            let body = body_for_load.clone();
                                                            view! {
                                                                <Btn
                                                                    variant="ghost"
                                                                    size="sm"
                                                                    on_click=Callback::new(move |_| cb.run(body.clone()))
                                                                >"load"</Btn>
                                                            }
                                                        })}
                                                        <Btn
                                                            variant="bare"
                                                            size="sm"
                                                            on_click=Callback::new(move |_| {
                                                                let id = id_for_del.clone();
                                                                spawn_local(async move {
                                                                    let _ = presets_api::delete_preset(&id).await;
                                                                    tick.update(|t| *t += 1);
                                                                });
                                                            })
                                                        >"delete"</Btn>
                                                    </div>
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

            <p class="muted xsmall" style="margin-top:14px; text-align:center">
                "// save a config with '+ save preset' in the New-game wizard, or import raw JSON above."
            </p>
        </div>
    }
}
