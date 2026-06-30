//! New-game wizard — port of `hifi/newgame.jsx`. Five steps
//! (Map / Civilization / Rules / Players / Review). All step state
//! lives on a single `WizardState` provided through context, so the
//! Review step's "⌬ Generate world" CTA posts the user's actual
//! selections to `POST /api/v1/games`.

use std::sync::Arc;

use leptos::prelude::*;
use serde::Serialize;
use wasm_bindgen_futures::spawn_local;

use crate::components::api::games as games_api;
use crate::components::api::presets as presets_api;
use crate::components::{
    Btn, MiniMap, Panel, PanelHead, Popup, PopupActions, PopupBody, PopupList,
    PopupListItem, PopupSize, PopupTrigger, Segmented, Slider, Tag, Toggle,
    segmented::Segment, slider::FormatFn,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Step {
    Map,
    Civ,
    Rules,
    Players,
    Review,
}

impl Step {
    const ALL: &'static [Step] = &[Step::Map, Step::Civ, Step::Rules, Step::Players, Step::Review];

    fn label(self) -> &'static str {
        match self {
            Step::Map => "Map",
            Step::Civ => "Civilization",
            Step::Rules => "Rules",
            Step::Players => "Players",
            Step::Review => "Review",
        }
    }

    fn idx(self) -> usize {
        Step::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    fn next(self) -> Option<Step> {
        Step::ALL.get(self.idx() + 1).copied()
    }

    fn prev(self) -> Option<Step> {
        if self.idx() == 0 { None } else { Step::ALL.get(self.idx() - 1).copied() }
    }
}

/// Hoisted wizard form state. Every step reads + writes to the same
/// signals so the Review step can post the user's actual selections
/// to `POST /api/v1/games` instead of static defaults.
///
/// All fields are `RwSignal`, which is `Copy` in Leptos 0.7 — so
/// `WizardState` itself is `Copy`/`Clone` and threads cheaply
/// through context.
#[derive(Copy, Clone)]
struct WizardState {
    // Map step.
    map_type: RwSignal<String>,
    map_size: RwSignal<String>,
    advanced: RwSignal<bool>,
    // Civ step.
    selected_leader: RwSignal<String>,
    selected_civ: RwSignal<String>,
    // Rules step — difficulty / pace / world dynamics.
    difficulty: RwSignal<String>,
    starting_era: RwSignal<String>,
    game_speed: RwSignal<String>,
    ai_personality: RwSignal<String>,
    disasters: RwSignal<i32>,
    barbarians: RwSignal<i32>,
    city_states: RwSignal<i32>,
    ai_aggression: RwSignal<i32>,
    /// One Toggle signal per row in `VICTORY_CONDITIONS` (defined
    /// further down). Hoisted here so StepReview can summarise them.
    victory: [RwSignal<bool>; 6],
    // Players step.
    timer: RwSignal<String>,
    simultaneous: RwSignal<bool>,
    private_game: RwSignal<bool>,
    cross_play: RwSignal<bool>,
}

impl WizardState {
    fn new() -> Self {
        Self {
            map_type: RwSignal::new("continents".into()),
            map_size: RwSignal::new("std".into()),
            advanced: RwSignal::new(false),
            selected_leader: RwSignal::new("Saladin".into()),
            selected_civ: RwSignal::new("Arabia".into()),
            difficulty: RwSignal::new("prince".into()),
            starting_era: RwSignal::new("ancient".into()),
            game_speed: RwSignal::new("std".into()),
            ai_personality: RwSignal::new("historic".into()),
            disasters: RwSignal::new(2),
            barbarians: RwSignal::new(2),
            city_states: RwSignal::new(12),
            ai_aggression: RwSignal::new(50),
            victory: [
                RwSignal::new(true),  // Science
                RwSignal::new(true),  // Culture
                RwSignal::new(true),  // Domination
                RwSignal::new(true),  // Religion
                RwSignal::new(false), // Diplomacy (off by default per JSX)
                RwSignal::new(true),  // Score
            ],
            timer: RwSignal::new("off".into()),
            simultaneous: RwSignal::new(false),
            private_game: RwSignal::new(true),
            cross_play: RwSignal::new(true),
        }
    }

    /// Build the `CreateGameBody` the lobby's `POST /api/v1/games`
    /// expects. Reads every signal current.
    #[allow(clippy::wrong_self_convention)]
    fn to_create_body(&self) -> games_api::CreateGameBody {
        let leader = self.selected_leader.get();
        let civ = self.selected_civ.get();
        // Players-human is fixed at 1 today (the design's open invite
        // slot doesn't promote to a real human yet); AI count is the
        // remaining slots in PLAYERS minus the human + open slot.
        let players_ai = (PLAYERS.len() as u32).saturating_sub(2);
        // Seed: deterministic per (leader, civ, map_size) so the same
        // wizard config produces the same world. Real seed-input UX
        // belongs in the StepMap advanced panel — pending.
        let seed = format!("0x{leader:0>4}·{civ:0>4}·{}", self.map_size.get());
        games_api::CreateGameBody {
            name: format!("{leader}'s {civ}"),
            leader: leader.clone(),
            civ_id: civ.to_lowercase(),
            difficulty: self.difficulty.get(),
            players_human: 1,
            players_ai,
            map_type: self.map_type.get(),
            map_size: self.map_size.get(),
            seed,
        }
    }

    /// Snapshot every wizard signal into a serializable preset body.
    /// Stored opaquely by `/api/v1/presets`; richer than
    /// `CreateGameBody` because it also carries the rules / dynamics /
    /// turn-mode fields the create-game route doesn't consume yet, so
    /// a future "load preset → wizard" path can round-trip the full
    /// configuration.
    fn to_preset(&self) -> WizardPreset {
        let victory = VICTORY_CONDITIONS
            .iter()
            .enumerate()
            .filter(|(i, _)| self.victory[*i].get())
            .map(|(_, (name, _, _))| name.to_string())
            .collect();
        WizardPreset {
            map_type: self.map_type.get(),
            map_size: self.map_size.get(),
            advanced: self.advanced.get(),
            leader: self.selected_leader.get(),
            civ: self.selected_civ.get(),
            difficulty: self.difficulty.get(),
            starting_era: self.starting_era.get(),
            game_speed: self.game_speed.get(),
            ai_personality: self.ai_personality.get(),
            disasters: self.disasters.get(),
            barbarians: self.barbarians.get(),
            city_states: self.city_states.get(),
            ai_aggression: self.ai_aggression.get(),
            victory,
            timer: self.timer.get(),
            simultaneous: self.simultaneous.get(),
            private_game: self.private_game.get(),
            cross_play: self.cross_play.get(),
        }
    }

    /// Default preset name — mirrors the generated game name so a
    /// saved config reads recognisably in the Presets tab.
    fn default_preset_name(&self) -> String {
        format!("{}'s {}", self.selected_leader.get(), self.selected_civ.get())
    }
}

/// Serializable snapshot of the whole wizard form. The `presets`
/// store treats `body_json` as opaque text, so the shape lives here
/// next to the producer rather than in the protocol crate.
#[derive(Serialize)]
struct WizardPreset {
    map_type: String,
    map_size: String,
    advanced: bool,
    leader: String,
    civ: String,
    difficulty: String,
    starting_era: String,
    game_speed: String,
    ai_personality: String,
    disasters: i32,
    barbarians: i32,
    city_states: i32,
    ai_aggression: i32,
    victory: Vec<String>,
    timer: String,
    simultaneous: bool,
    private_game: bool,
    cross_play: bool,
}

#[component]
pub fn NewGame(#[prop(optional)] on_generated: Option<Callback<String>>) -> impl IntoView {
    let step = RwSignal::new(Step::Map);
    provide_context(WizardState::new());

    // Make the on_generated callback available to StepReview via context
    // so the deeply-nested CTA can fire it without a prop chain.
    if let Some(cb) = on_generated {
        provide_context(GeneratedCb(cb));
    }

    view! {
        <div style="flex:1; display:flex; flex-direction:column; min-height:0">
            <div class="content-header">
                <div class="title">"New game"</div>
                <span class="crumbs">"// procedural worldgen"</span>
                <div class="actions">
                    <Btn variant="ghost" size="sm">"presets"</Btn>
                </div>
            </div>

            <StepStrip step=step />

            <div style="flex:1; overflow:auto; padding-bottom:12px">
                {move || match step.get() {
                    Step::Map => view! { <StepMap /> }.into_any(),
                    Step::Civ => view! { <StepCiv /> }.into_any(),
                    Step::Rules => view! { <StepRules /> }.into_any(),
                    Step::Players => view! { <StepPlayers /> }.into_any(),
                    Step::Review => view! { <StepReview /> }.into_any(),
                }}
            </div>

            <div class="wizard-footer">
                <div style="display:flex; gap:8px; align-items:center">
                    <Btn variant="ghost" size="sm"
                         disabled=Signal::derive(move || step.get().prev().is_none())
                         on_click=Callback::new(move |_| {
                             if let Some(p) = step.get().prev() { step.set(p); }
                         })>
                        "← back"
                    </Btn>
                    <SavePreset />
                </div>
                <span>
                    <span class="kbd">"⏎"</span>" next · "
                    <span class="kbd">"⌘K"</span>" jump · "
                    <span class="kbd">"esc"</span>" cancel"
                </span>
                {move || if step.get() == Step::Review {
                    view! { <Btn variant="accent">"⌬ generate"</Btn> }.into_any()
                } else {
                    view! {
                        <Btn variant="primary"
                             on_click=Callback::new(move |_| {
                                 if let Some(n) = step.get().next() { step.set(n); }
                             })>
                            "next →"
                        </Btn>
                    }.into_any()
                }}
            </div>
        </div>
    }
}

#[component]
fn StepStrip(step: RwSignal<Step>) -> impl IntoView {
    let total = Step::ALL.len();
    view! {
        <div class="wizard-steps">
            {Step::ALL.iter().copied().enumerate().map(|(i, s)| {
                let cur_idx = move || step.get().idx();
                let class = move || {
                    if i < cur_idx() { "step done" }
                    else if i == cur_idx() { "step current" }
                    else { "step" }
                };
                view! {
                    <button class=class on:click=move |_| step.set(s)>
                        <span class="num">{(i + 1).to_string()}</span>
                        <span>{s.label()}</span>
                    </button>
                    {(i < total - 1).then(|| view! { <span class="arrow">"›"</span> })}
                }
            }).collect::<Vec<_>>()}
            <span style="margin-left:auto" class="muted xsmall">
                {move || format!("step {} of {total}", step.get().idx() + 1)}
            </span>
        </div>
    }
}

// (StepPlaceholder removed — every wizard step now has a real port.)

/// Carrier for the optional on_generated callback so deeply-nested
/// children (StepReview) can fire it from context without a prop
/// chain through every Step component.
#[derive(Clone)]
struct GeneratedCb(Callback<String>);

// ─────────────────────────── Save-preset shortcut ──────────────────────────────

#[derive(Clone, PartialEq, Default)]
enum SaveState {
    #[default]
    Idle,
    Pending,
    Saved,
    Error(String),
}

/// "+ save preset" button + inline name form. Reads `WizardState`
/// from context, serialises the live configuration, and POSTs it to
/// `/api/v1/presets` without leaving the wizard. Rendered in the
/// shared wizard footer so it's reachable from every step.
#[component]
fn SavePreset() -> impl IntoView {
    let state = expect_context::<WizardState>();
    let open = RwSignal::new(false);
    let name = RwSignal::new(String::new());
    let status = RwSignal::new(SaveState::Idle);

    let toggle = move |_| {
        let now_open = !open.get_untracked();
        open.set(now_open);
        if now_open {
            // Seed the field with the default name on first open.
            if name.get_untracked().trim().is_empty() {
                name.set(state.default_preset_name());
            }
            status.set(SaveState::Idle);
        }
    };

    let on_save = move |_| {
        if matches!(status.get_untracked(), SaveState::Pending) {
            return;
        }
        let n = name.get_untracked().trim().to_string();
        if n.is_empty() {
            status.set(SaveState::Error("name required".into()));
            return;
        }
        let body_json = match serde_json::to_string_pretty(&state.to_preset()) {
            Ok(s) => s,
            Err(e) => {
                status.set(SaveState::Error(e.to_string()));
                return;
            }
        };
        status.set(SaveState::Pending);
        spawn_local(async move {
            match presets_api::create(n, body_json).await {
                Ok(_) => status.set(SaveState::Saved),
                Err(e) => status.set(SaveState::Error(e.to_string())),
            }
        });
    };

    view! {
        <span style="position:relative; display:inline-block">
            <Btn
                variant="ghost"
                size="sm"
                on_click=Callback::new(toggle)
            >
                {move || if open.get() { "× save preset" } else { "+ save preset" }}
            </Btn>
            {move || open.get().then(|| view! {
                <div
                    class="panel"
                    style="position:absolute; bottom:calc(100% + 6px); left:0; z-index:50; \
                           width:260px; padding:10px; box-shadow:0 6px 24px rgba(0,0,0,.18)"
                >
                    <div class="muted xsmall" style="margin-bottom:6px">
                        "Save the current wizard configuration as a preset."
                    </div>
                    <input
                        class="input"
                        style="width:100%; margin-bottom:8px"
                        placeholder="preset name"
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
                    <Btn
                        variant="accent"
                        size="sm"
                        class="block"
                        disabled=Signal::derive(move || status.get() == SaveState::Pending)
                        on_click=Callback::new(on_save)
                    >
                        {move || match status.get() {
                            SaveState::Pending => "Saving…",
                            SaveState::Saved => "Saved ✓",
                            _ => "Save preset",
                        }}
                    </Btn>
                    {move || match status.get() {
                        SaveState::Error(msg) => view! {
                            <p class="xsmall" style="color:var(--accent); margin:6px 0 0">{msg}</p>
                        }.into_any(),
                        SaveState::Saved => view! {
                            <p class="muted xsmall" style="margin:6px 0 0">
                                "// stored · find it under the Presets tab"
                            </p>
                        }.into_any(),
                        _ => view! { <></> }.into_any(),
                    }}
                </div>
            })}
        </span>
    }
}

// ─────────────────────────────── Step: map ────────────────────────────────────

#[component]
fn StepMap() -> impl IntoView {
    let state = expect_context::<WizardState>();
    let map_type = state.map_type;
    let map_size = state.map_size;
    let advanced = state.advanced;

    let map_type_opts = Signal::derive(|| {
        ["continents", "pangaea", "archipelago", "fractal", "custom"]
            .iter().map(|s| Segment::from_str(s)).collect()
    });
    let map_size_opts = Signal::derive(|| {
        ["duel", "tiny", "small", "std", "large", "huge"]
            .iter().map(|s| Segment::from_str(s)).collect()
    });

    view! {
        <div class="wizard-body">
            <Panel flush=true>
                <PanelHead
                    title="Map & world".to_string()
                    sub="// procgen parameters"
                />
                <div class="panel-body">
                    <div class="param-row stack">
                        <div class="label">
                            <Popup
                                title="map type"
                                content=Arc::new(|| view! {
                                    <PopupBody>
                                        <p><strong>"Continents"</strong>" — 2-3 large landmasses with ocean separation."</p>
                                        <p><strong>"Pangaea"</strong>" — one supercontinent."</p>
                                        <p><strong>"Archipelago"</strong>" — many small islands."</p>
                                        <p><strong>"Fractal"</strong>" — Perlin-noise seeded; unpredictable shapes."</p>
                                        <p><strong>"Custom"</strong>" — paste a seed or import from JSON."</p>
                                    </PopupBody>
                                }.into_any())
                            >
                                <span class="trigger">"map type"</span>
                            </Popup>
                        </div>
                        <div class="control">
                            <Segmented options=map_type_opts value=map_type />
                        </div>
                    </div>

                    <div class="param-row stack">
                        <div class="label">
                            <Popup
                                title="map size"
                                content=Arc::new(|| view! {
                                    <PopupBody>
                                        <p>"Tile dimensions of the world."</p>
                                        <div class="kv xsmall">
                                            <span class="k">"duel"</span><span>"44×26"</span>
                                            <span class="k">"tiny"</span><span>"60×38"</span>
                                            <span class="k">"small"</span><span>"74×46"</span>
                                            <span class="k">"std"</span><span>"84×54"</span>
                                            <span class="k">"large"</span><span>"96×60"</span>
                                            <span class="k">"huge"</span><span>"106×66"</span>
                                        </div>
                                    </PopupBody>
                                }.into_any())
                            >
                                <span class="trigger">"map size"</span>
                            </Popup>
                            <span class="muted xsmall" style="text-transform:none; letter-spacing:0; margin-left:6px">
                                "standard · 84×54"
                            </span>
                        </div>
                        <div class="control">
                            <Segmented options=map_size_opts value=map_size />
                        </div>
                    </div>

                    <div class="param-row">
                        <div class="label">"advanced parameters"</div>
                        <div class="control">
                            <Toggle on=advanced
                                    on_change=Callback::new(move |v| advanced.set(v)) />
                        </div>
                        <div class="value muted xsmall">
                            "world age · sea level · temperature · rainfall · resources · seed"
                        </div>
                    </div>
                </div>
            </Panel>

            <div class="col">
                <Panel flush=true>
                    <PanelHead
                        title="Preview".to_string()
                        sub="// regenerable"
                    />
                    <div style="padding:1px">
                        <div class="map-preview">
                            <span class="corner">"// 84×54 · continents · seed 0xCAFE…"</span>
                            <span class="corner tr">"⟳"</span>
                            <MiniMap seed=42 style="position:absolute; inset:0; width:100%; height:100%" />
                        </div>
                    </div>
                    <div class="row between center-y" style="padding:10px 14px; border-top:1px solid var(--hairline); font-size:var(--fs-xs)">
                        <span class="muted">"tiles 4536 · land 47% · climate temperate"</span>
                        <Btn variant="ghost" size="sm">"⟳ regenerate"</Btn>
                    </div>
                </Panel>

                <Panel>
                    <div class="h3" style="margin-bottom:8px">"Hint"</div>
                    <p class="muted small" style="margin:0">
                        "The advanced toggle exposes 12+ procgen parameters. Hover any "
                        "underlined label to see what it does."
                    </p>
                </Panel>
            </div>
        </div>
    }
}

// ────────────────────────────── Step: review ──────────────────────────────────

fn map_size_dims(size: &str) -> &'static str {
    match size {
        "duel" => "44×26",
        "tiny" => "60×38",
        "small" => "74×46",
        "large" => "96×60",
        "huge" => "106×66",
        _ => "84×54",
    }
}

fn slider_label_disasters(v: i32) -> &'static str {
    ["off", "light", "std", "heavy", "apocalyptic"][v.clamp(0, 4) as usize]
}

fn slider_label_barbs(v: i32) -> &'static str {
    ["off", "rare", "std", "raging", "horde"][v.clamp(0, 4) as usize]
}

fn slider_label_aggr(v: i32) -> &'static str {
    if v < 34 { "passive" } else if v > 66 { "warlike" } else { "balanced" }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum GenerateState {
    #[default]
    Idle,
    Pending,
    Error(String),
}

#[component]
fn StepReview() -> impl IntoView {
    let gen_state = RwSignal::new(GenerateState::Idle);
    let gen_cb = use_context::<GeneratedCb>();
    let state = expect_context::<WizardState>();

    let on_generate = move |_| {
        if matches!(gen_state.get_untracked(), GenerateState::Pending) {
            return;
        }
        gen_state.set(GenerateState::Pending);
        let body = state.to_create_body();
        let cb = gen_cb.clone();
        spawn_local(async move {
            match games_api::create(body).await {
                Ok(game) => {
                    if let Some(GeneratedCb(cb)) = cb {
                        cb.run(game.game_id);
                    } else {
                        gen_state.set(GenerateState::Idle);
                    }
                }
                Err(e) => gen_state.set(GenerateState::Error(e.to_string())),
            }
        });
    };
    let pending = Signal::derive(move || matches!(gen_state.get(), GenerateState::Pending));

    // Build the summary rows live from WizardState. Replaces the
    // static REVIEW_ROWS table.
    let summary_rows = move || -> Vec<(String, String)> {
        let leader = state.selected_leader.get();
        let civ = state.selected_civ.get();
        let map = format!(
            "{} · {} · {}",
            state.map_type.get(),
            state.map_size.get(),
            map_size_dims(&state.map_size.get()),
        );
        let world = if state.advanced.get() {
            "advanced overrides applied".to_string()
        } else {
            "default world params".to_string()
        };
        let civilization = format!("{leader} / {civ}");
        let difficulty_line = format!(
            "{} · {} speed · {} era",
            state.difficulty.get(),
            state.game_speed.get(),
            state.starting_era.get(),
        );
        let victory: String = VICTORY_CONDITIONS
            .iter()
            .enumerate()
            .filter(|(i, _)| state.victory[*i].get())
            .map(|(_, (n, _, _))| n.to_lowercase())
            .collect::<Vec<_>>()
            .join(" · ");
        let dynamics = format!(
            "disasters {} · barbs {} · {} city-states · AI {}",
            slider_label_disasters(state.disasters.get()),
            slider_label_barbs(state.barbarians.get()),
            state.city_states.get(),
            slider_label_aggr(state.ai_aggression.get()),
        );
        let players_count = (PLAYERS.len() as u32).saturating_sub(2);
        let players = format!("1 human + 1 invite pending + {players_count} AI");
        let turn_mode = format!(
            "{} · {} · {}",
            if state.simultaneous.get() { "simultaneous" } else { "play-by-turn" },
            if state.private_game.get() { "invite-only" } else { "public" },
            if state.cross_play.get() { "web · API" } else { "web only" },
        );
        let timer_label = state.timer.get();
        let seed = format!("0x{leader:0>4}·{civ:0>4}·{}", state.map_size.get());
        vec![
            ("map".into(), map),
            ("seed".into(), seed),
            ("world".into(), world),
            ("civilization".into(), civilization),
            ("difficulty".into(), difficulty_line),
            ("victory".into(), if victory.is_empty() { "none — disable all and you lose forever".into() } else { victory }),
            ("dynamics".into(), dynamics),
            ("players".into(), players),
            ("turn mode".into(), format!("{turn_mode} · timer {timer_label}")),
        ]
    };

    view! {
        <div class="wizard-body">
            <Panel flush=true>
                <PanelHead
                    title="Summary".to_string()
                    sub="// last chance to tweak"
                />
                <div class="panel-body">
                    {move || summary_rows().into_iter().map(|(k, v)| view! {
                        <div class="param-row">
                            <div class="label">{k}</div>
                            <div class="control" style="font-size:var(--fs-sm)">{v}</div>
                            <div class="value">
                                <Btn variant="bare" size="xs">"edit"</Btn>
                            </div>
                        </div>
                    }).collect::<Vec<_>>()}
                </div>
            </Panel>

            <div class="col">
                <Panel flush=true>
                    <PanelHead
                        title="Final preview".to_string()
                        sub="// world will be locked at generate"
                    />
                    <div style="padding:1px">
                        <div class="map-preview">
                            <MiniMap seed=42 style="position:absolute; inset:0; width:100%; height:100%" />
                        </div>
                    </div>
                </Panel>
                <Panel class="">
                    <p class="small" style="margin-top:0; margin-bottom:12px">
                        "Generation deterministically builds the world from your seed. "
                        "You can copy the seed to recreate this exact map elsewhere."
                    </p>
                    <Btn
                        variant="accent"
                        size="lg"
                        class="block"
                        disabled=pending
                        on_click=Callback::new(on_generate)
                    >
                        {move || if pending.get() { "Generating…" } else { "⌬  Generate world" }}
                    </Btn>
                    {move || match gen_state.get() {
                        GenerateState::Error(msg) => view! {
                            <p class="xsmall" style="color:var(--accent); margin-top:8px">{msg}</p>
                        }.into_any(),
                        _ => view! {
                            <p class="muted xsmall" style="text-align:center; margin-top:10px; margin-bottom:0">
                                "// calls POST /api/v1/games · returns game_id · routes back to Ongoing"
                            </p>
                        }.into_any(),
                    }}
                </Panel>
            </div>
        </div>
    }
}

// ─────────────────────────────── Step: civ ────────────────────────────────────

/// One row in the civ picker. Static for now; the real catalogue lives in
/// `libciv` and will hand-shake with the lobby once the games-index ships.
#[derive(Copy, Clone)]
struct CivPick {
    leader: &'static str,
    civ: &'static str,
    trait_: &'static str,
    unique_unit: &'static str,
    unique_building: &'static str,
    leader_ability: &'static str,
    civ_ability: &'static str,
}

const CIVS: &[CivPick] = &[
    CivPick { leader: "Saladin",   civ: "Arabia",  trait_: "Trade & faith",     unique_unit: "Mamluk",         unique_building: "Madrasa",     leader_ability: "Righteousness of the Faith", civ_ability: "The Last Prophet" },
    CivPick { leader: "Trajan",    civ: "Rome",    trait_: "Expansionist",      unique_unit: "Legion",         unique_building: "Bath",        leader_ability: "Trajan's Column",            civ_ability: "All Roads Lead to Rome" },
    CivPick { leader: "Catherine", civ: "Russia",  trait_: "Wide / faith",      unique_unit: "Cossack",        unique_building: "Lavra",       leader_ability: "The Grand Embassy",          civ_ability: "Mother Russia" },
    CivPick { leader: "Cleopatra", civ: "Egypt",   trait_: "Wonders / trade",   unique_unit: "Maryannu Chariot Archer", unique_building: "Sphinx", leader_ability: "Mediterranean's Bride",   civ_ability: "Iteru" },
    CivPick { leader: "Hojo",      civ: "Japan",   trait_: "Coastal / military", unique_unit: "Samurai",       unique_building: "Electronics Factory", leader_ability: "Divine Wind",        civ_ability: "Meiji Restoration" },
    CivPick { leader: "Gandhi",    civ: "India",   trait_: "Religion / peace",  unique_unit: "Varu",           unique_building: "Stepwell",    leader_ability: "Satyagraha",                 civ_ability: "Dharma" },
    CivPick { leader: "Pedro II",  civ: "Brazil",  trait_: "Cultural",          unique_unit: "Minas Geraes",   unique_building: "Street Carnival", leader_ability: "Magnanimous",            civ_ability: "Amazon" },
    CivPick { leader: "Random",    civ: "?",       trait_: "surprise me",       unique_unit: "—",              unique_building: "—",           leader_ability: "—",                          civ_ability: "—" },
];

#[component]
fn StepCiv() -> impl IntoView {
    let state = expect_context::<WizardState>();
    let selected = state.selected_leader;

    view! {
        <div class="wizard-body single">
            <Panel flush=true>
                <PanelHead
                    title="Pick your civilization".to_string()
                    sub="// hover any leader to see their unique units & abilities"
                />
                <div class="panel-body">
                    <div style="display:grid; grid-template-columns:repeat(auto-fill, minmax(220px, 1fr)); gap:8px">
                        {CIVS.iter().copied().map(|c| {
                            let leader = c.leader;
                            let civ = c.civ;
                            let trait_ = c.trait_;
                            let leader_initial = leader.chars().next().unwrap_or('?').to_string();

                            let pick_leader = leader.to_string();
                            let pick_civ = civ.to_string();
                            let pick_for_class = leader.to_string();

                            let card_panel_style = move || {
                                let is_sel = selected.get() == pick_for_class;
                                if is_sel {
                                    "cursor:pointer; padding:12px; width:100%; border-color:var(--accent); background:var(--accent-soft)"
                                } else {
                                    "cursor:pointer; padding:12px; width:100%; border-color:var(--hairline); background:var(--paper)"
                                }
                            };

                            let popup_content = Arc::new(move || view! {
                                <PopupBody>
                                    <div style="font-weight:600; font-size:var(--fs-md)">
                                        {leader} " · " {civ}
                                    </div>
                                    <p class="muted xsmall" style="margin-bottom:8px">
                                        {trait_}
                                    </p>
                                    <div class="kv xsmall">
                                        <span class="k">"unique unit"</span><span>{c.unique_unit}</span>
                                        <span class="k">"unique bldg"</span><span>{c.unique_building}</span>
                                        <span class="k">"leader ability"</span><span>{c.leader_ability}</span>
                                        <span class="k">"civ ability"</span><span>{c.civ_ability}</span>
                                    </div>
                                </PopupBody>
                                <PopupActions right=true>
                                    <Btn variant="ghost" size="sm">"view full sheet"</Btn>
                                    <Btn variant="primary" size="sm">"select"</Btn>
                                </PopupActions>
                            }.into_any());

                            view! {
                                <Popup
                                    title="civ sheet"
                                    content=popup_content
                                >
                                    <div
                                        class="panel"
                                        style=card_panel_style
                                        on:click=move |_| {
                                            selected.set(pick_leader.clone());
                                            state.selected_civ.set(pick_civ.clone());
                                        }
                                    >
                                        <div class="row gap-sm">
                                            <div style="width:40px; height:40px; \
                                                        background:var(--ink); color:var(--paper); \
                                                        display:grid; place-items:center; \
                                                        font-family:var(--font-serif); font-size:22px">
                                                {leader_initial}
                                            </div>
                                            <div style="min-width:0">
                                                <div style="font-weight:600">{leader}</div>
                                                <div class="muted xsmall">{civ}</div>
                                                <div class="xsmall" style="margin-top:2px">{trait_}</div>
                                            </div>
                                        </div>
                                    </div>
                                </Popup>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            </Panel>
        </div>
    }
}

// ─────────────────────────────── Step: rules ──────────────────────────────────

const VICTORY_CONDITIONS: &[(&str, &str, bool)] = &[
    ("Science",    "Launch a colony to a habitable exoplanet.",                                  true),
    ("Culture",    "Attract more tourists than any other civ has domestic visitors.",            true),
    ("Domination", "Capture every other civ's original capital.",                                true),
    ("Religion",   "Convert every other civ to your founded religion.",                          true),
    ("Diplomacy",  "Earn the most diplomatic favor in the World Congress.",                      false),
    ("Score",      "Highest score when the time runs out.",                                      true),
];

#[component]
fn StepRules() -> impl IntoView {
    let state = expect_context::<WizardState>();
    let difficulty = state.difficulty;
    let starting_era = state.starting_era;
    let game_speed = state.game_speed;
    let ai_personality = state.ai_personality;
    let disasters = state.disasters;
    let barbarians = state.barbarians;
    let city_states = state.city_states;
    let ai_aggression = state.ai_aggression;
    let victory_signals: Vec<RwSignal<bool>> = state.victory.to_vec();

    let difficulty_opts = Signal::derive(|| {
        ["settler", "chieftain", "warlord", "prince", "king", "emperor", "deity"]
            .iter().map(|s| Segment::from_str(s)).collect()
    });
    let era_opts = Signal::derive(|| {
        ["ancient", "classical", "medieval", "renaissance", "industrial"]
            .iter().map(|s| Segment::from_str(s)).collect()
    });
    let speed_opts = Signal::derive(|| {
        ["online", "quick", "std", "epic", "marathon"]
            .iter().map(|s| Segment::from_str(s)).collect()
    });
    let personality_opts = Signal::derive(|| {
        ["historic", "random", "scripted"].iter().map(|s| Segment::from_str(s)).collect()
    });

    // Categorical formatters for the world-dynamics sliders.
    let disaster_fmt: FormatFn = Arc::new(|v: i32| {
        ["off", "light", "std", "heavy", "apocalyptic"][v.clamp(0, 4) as usize].to_string()
    });
    let barb_fmt: FormatFn = Arc::new(|v: i32| {
        ["off", "rare", "std", "raging", "horde"][v.clamp(0, 4) as usize].to_string()
    });
    let aggr_fmt: FormatFn = Arc::new(|v: i32| {
        if v < 34 { "passive" } else if v > 66 { "warlike" } else { "balanced" }.to_string()
    });

    view! {
        <div class="wizard-body">
            <Panel flush=true>
                <PanelHead title="Difficulty & pace".to_string() />
                <div class="panel-body">
                    <div class="param-row stack">
                        <div class="label">
                            <Popup
                                title="difficulty"
                                content=Arc::new(|| view! {
                                    <PopupBody>
                                        <p>"Affects AI bonuses, barbarian aggression, and yield modifiers."</p>
                                        <div class="kv xsmall" style="margin-top:6px">
                                            <span class="k">"settler"</span><span>"−40% AI yields"</span>
                                            <span class="k">"prince"</span><span>"baseline"</span>
                                            <span class="k">"deity"</span><span>"+50% AI yields"</span>
                                        </div>
                                    </PopupBody>
                                }.into_any())
                            >
                                <span class="trigger">"difficulty"</span>
                            </Popup>
                        </div>
                        <div class="control"><Segmented options=difficulty_opts value=difficulty /></div>
                    </div>

                    <div class="param-row stack">
                        <div class="label">"starting era"</div>
                        <div class="control"><Segmented options=era_opts value=starting_era /></div>
                    </div>

                    <div class="param-row stack">
                        <div class="label">
                            <Popup
                                title="game speed"
                                content=Arc::new(|| view! {
                                    <PopupBody>
                                        <p>"Game speed scales tech, civic, production, and unit costs uniformly."</p>
                                    </PopupBody>
                                }.into_any())
                            >
                                <span class="trigger">"game speed"</span>
                            </Popup>
                        </div>
                        <div class="control"><Segmented options=speed_opts value=game_speed /></div>
                        <div class="value">"standard"</div>
                    </div>

                    <hr class="divider" />
                    <div class="h3" style="margin-bottom:10px">"Victory conditions"</div>
                    {VICTORY_CONDITIONS.iter().enumerate().map(|(i, (name, desc, _))| {
                        let sig = victory_signals[i];
                        let lower = name.to_lowercase();
                        let desc_static: &'static str = desc;
                        view! {
                            <div class="param-row">
                                <div class="label">
                                    <Popup
                                        title=name.to_string()
                                        content=Arc::new(move || view! {
                                            <PopupBody><p>{desc_static}</p></PopupBody>
                                        }.into_any())
                                    >
                                        <span class="trigger">{lower}</span>
                                    </Popup>
                                </div>
                                <div class="control">
                                    <Toggle on=sig />
                                </div>
                                <div class="value muted xsmall">
                                    {move || if sig.get() { "enabled" } else { "off" }}
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </Panel>

            <Panel flush=true>
                <PanelHead title="World dynamics".to_string() />
                <div class="panel-body">
                    <div class="param-row">
                        <div class="label">
                            <Popup
                                title="disasters"
                                content=Arc::new(|| view! {
                                    <PopupBody>
                                        <p>"Volcanoes, floods, droughts, blizzards. Higher intensity = more frequent & severe."</p>
                                    </PopupBody>
                                }.into_any())
                            >
                                <span class="trigger">"disasters"</span>
                            </Popup>
                        </div>
                        <div class="control">
                            <Slider value=disasters min=0 max=4 format=disaster_fmt />
                        </div>
                    </div>

                    <div class="param-row">
                        <div class="label">"barbarians"</div>
                        <div class="control">
                            <Slider value=barbarians min=0 max=4 format=barb_fmt />
                        </div>
                    </div>

                    <div class="param-row">
                        <div class="label">"city-states"</div>
                        <div class="control">
                            <Slider value=city_states min=0 max=24 />
                        </div>
                    </div>

                    <div class="param-row">
                        <div class="label">
                            <Popup
                                title="AI aggression"
                                content=Arc::new(|| view! {
                                    <PopupBody>
                                        <p>"Affects how often AI civs declare war, denounce, or accept peace."</p>
                                    </PopupBody>
                                }.into_any())
                            >
                                <span class="trigger">"AI aggression"</span>
                            </Popup>
                        </div>
                        <div class="control">
                            <Slider value=ai_aggression format=aggr_fmt />
                        </div>
                    </div>

                    <div class="param-row">
                        <div class="label">
                            <Popup
                                title="AI personality"
                                content=Arc::new(|| view! {
                                    <PopupBody>
                                        <p><strong>"Historic"</strong>" — each leader behaves like their flavor text."</p>
                                        <p><strong>"Random"</strong>" — personalities reshuffled each game."</p>
                                        <p><strong>"Scripted"</strong>" — load a JSON personality pack."</p>
                                    </PopupBody>
                                }.into_any())
                            >
                                <span class="trigger">"AI personality"</span>
                            </Popup>
                        </div>
                        <div class="control"><Segmented options=personality_opts value=ai_personality /></div>
                        <div class="value">{move || ai_personality.get()}</div>
                    </div>
                </div>
            </Panel>
        </div>
    }
}

// ─────────────────────────────── Step: players ────────────────────────────────

#[derive(Copy, Clone)]
enum SlotKind {
    Human,
    Open,
    Ai,
}

impl SlotKind {
    fn label(self) -> &'static str {
        match self {
            SlotKind::Human => "human",
            SlotKind::Open => "open",
            SlotKind::Ai => "ai",
        }
    }

    fn tag_variant(self) -> &'static str {
        match self {
            SlotKind::Human => "accent-soft",
            SlotKind::Open => "",
            SlotKind::Ai => "dim",
        }
    }
}

#[derive(Copy, Clone)]
struct PlayerRow {
    name: &'static str,
    civ: &'static str,
    kind: SlotKind,
    you: bool,
    invite: bool,
}

const PLAYERS: &[PlayerRow] = &[
    PlayerRow { name: "Alice (you)", civ: "Arabia · Saladin",  kind: SlotKind::Human, you: true,  invite: false },
    PlayerRow { name: "—",           civ: "—",                 kind: SlotKind::Open,  you: false, invite: true  },
    PlayerRow { name: "AI",          civ: "Rome · Trajan",     kind: SlotKind::Ai,    you: false, invite: false },
    PlayerRow { name: "AI",          civ: "Russia · Catherine",kind: SlotKind::Ai,    you: false, invite: false },
    PlayerRow { name: "AI",          civ: "Random",            kind: SlotKind::Ai,    you: false, invite: false },
    PlayerRow { name: "AI",          civ: "Random",            kind: SlotKind::Ai,    you: false, invite: false },
    PlayerRow { name: "AI",          civ: "Random",            kind: SlotKind::Ai,    you: false, invite: false },
    PlayerRow { name: "AI",          civ: "Random",            kind: SlotKind::Ai,    you: false, invite: false },
];

fn slot_class(p: &PlayerRow) -> &'static str {
    match (p.you, matches!(p.kind, SlotKind::Open)) {
        (true, _) => "slot you",
        (_, true) => "slot open",
        _ => "slot",
    }
}

fn invite_popup_content() -> AnyView {
    view! {
        <PopupBody>
            <p class="xsmall muted" style="margin-bottom:6px">
                "Paste any email, OpenID URL, atproto handle, or player ID:"
            </p>
            <input class="input mono" placeholder="alice@…  did:plc:…  0xA9C3·…" />
            <div class="row wrap gap-xs" style="margin-top:8px">
                <span class="xsmall muted" style="align-self:center; margin-right:4px">"recent:"</span>
                <button class="chip">"bob.bsky.social"</button>
                <button class="chip">"carol@…"</button>
                <button class="chip">"0xFE12·…"</button>
            </div>
        </PopupBody>
        <PopupActions right=true>
            <Btn variant="ghost" size="sm">"⎘ copy invite link"</Btn>
            <Btn variant="accent" size="sm">"send invite"</Btn>
        </PopupActions>
    }.into_any()
}

fn slot_menu_content() -> AnyView {
    let items = vec![
        PopupListItem::row("◔", "Change civ"),
        PopupListItem::row("⚙", "AI personality"),
        PopupListItem::row("↔", "Swap with…"),
        PopupListItem::sep(),
        PopupListItem::row("✕", "Remove slot"),
    ];
    view! { <PopupList items=items /> }.into_any()
}

#[component]
fn StepPlayers() -> impl IntoView {
    let state = expect_context::<WizardState>();
    let timer = state.timer;
    let simultaneous = state.simultaneous;
    let private_game = state.private_game;
    let cross_play = state.cross_play;

    let timer_opts = Signal::derive(|| {
        ["off", "5min", "10min", "30min", "24hr"].iter().map(|s| Segment::from_str(s)).collect()
    });

    let humans = PLAYERS.iter().filter(|p| matches!(p.kind, SlotKind::Human) || p.you).count();
    let ais = PLAYERS.iter().filter(|p| matches!(p.kind, SlotKind::Ai)).count();
    let sub = format!("// {humans}H · {ais}AI");

    view! {
        <div class="wizard-body">
            <Panel flush=true>
                // Inline panel-head — PanelHead doesn't accept a right slot
                // (one-off; most heads don't need it).
                <div class="panel-head">
                    <span class="title">"Players & slots"</span>
                    <span class="sub">{sub.clone()}</span>
                    <div style="margin-left:auto">
                        <Btn variant="ghost" size="sm">"+ slot"</Btn>
                    </div>
                </div>
                <div class="panel-body">
                    {PLAYERS.iter().enumerate().map(|(i, p)| {
                        let class = slot_class(p);
                        let kind_label = p.kind.label();
                        let tag_variant = p.kind.tag_variant();
                        let action = if p.invite {
                            view! {
                                <Popup
                                    title="Invite player"
                                    trigger=PopupTrigger::Click
                                    content=Arc::new(invite_popup_content)
                                >
                                    <Btn variant="primary" size="sm">"invite"</Btn>
                                </Popup>
                            }.into_any()
                        } else {
                            view! {
                                <Popup
                                    title="Slot"
                                    size=PopupSize::Narrow
                                    trigger=PopupTrigger::Click
                                    content=Arc::new(slot_menu_content)
                                >
                                    <Btn variant="ghost" size="sm">"···"</Btn>
                                </Popup>
                            }.into_any()
                        };
                        view! {
                            <div class=class>
                                <span class="num">{format!("#{}", i + 1)}</span>
                                <div style="min-width:0">
                                    <div class="row gap-sm center-y">
                                        <span class="name">{p.name}</span>
                                        <Tag variant=tag_variant>{kind_label}</Tag>
                                    </div>
                                    <div class="civ">{p.civ}</div>
                                </div>
                                {action}
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </Panel>

            <Panel flush=true>
                <PanelHead title="Turn mode".to_string() />
                <div class="panel-body">
                    <div class="param-row stack">
                        <div class="label">"turn timer"</div>
                        <div class="control"><Segmented options=timer_opts value=timer /></div>
                    </div>
                    <div class="param-row">
                        <div class="label">
                            <Popup
                                title="simultaneous"
                                content=Arc::new(|| view! {
                                    <PopupBody>
                                        <p>"All human players take their turns at the same time. Falls back to play-by-turn for AI phases."</p>
                                    </PopupBody>
                                }.into_any())
                            >
                                <span class="trigger">"simultaneous"</span>
                            </Popup>
                        </div>
                        <div class="control"><Toggle on=simultaneous /></div>
                        <div class="value muted xsmall">
                            {move || if simultaneous.get() { "simultaneous" } else { "play-by-turn" }}
                        </div>
                    </div>
                    <div class="param-row">
                        <div class="label">"private game"</div>
                        <div class="control"><Toggle on=private_game /></div>
                        <div class="value muted xsmall">
                            {move || if private_game.get() { "invite-only" } else { "public" }}
                        </div>
                    </div>
                    <div class="param-row">
                        <div class="label">"cross-play"</div>
                        <div class="control"><Toggle on=cross_play /></div>
                        <div class="value muted xsmall">
                            {move || if cross_play.get() { "web · API" } else { "web only" }}
                        </div>
                    </div>
                </div>
            </Panel>
        </div>
    }
}
