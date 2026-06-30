//! Top-level Leptos app component for the lobby SPA.
//!
//! Renders the `app-bar` chrome (brand · HI-FI pill · screen tabs · status
//! dot · `?` kbd hint) and a single `RwSignal<Screen>` that drives which
//! screen mounts below. The "Sign in & play →" CTA on the landing screen
//! switches to [`Screen::Login`]; navigating to `Screen::Menu` drops the
//! user into the [`crate::screens::MenuShell`] with a default tab.

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::components::api::me as me_api;
use crate::components::{PopupProvider, TweaksPanel};
use crate::screens::{
    Docs, Friends, Landing, Login, MenuShell, MenuTab, NewGame, OngoingGames, Presets, Profile,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Screen {
    Landing,
    Login,
    Menu,
}

/// App-level carrier for a preset the user asked to load from the
/// Presets tab. `NewGame` drains it on mount and applies it to the
/// fresh `WizardState`. Holds the raw `body_json` string.
#[derive(Copy, Clone)]
pub struct PendingPreset(pub RwSignal<Option<String>>);

impl Screen {
    fn label(self) -> &'static str {
        match self {
            Screen::Landing => "01 Landing",
            Screen::Login => "02 Login",
            Screen::Menu => "03 Menu",
        }
    }

    const ALL: &'static [Screen] = &[Screen::Landing, Screen::Login, Screen::Menu];
}

#[component]
pub fn App() -> impl IntoView {
    let screen = RwSignal::new(Screen::Landing);
    let menu_tab = RwSignal::new(MenuTab::Ongoing);
    let density = RwSignal::new("comfortable".to_string());
    crate::components::thumbnail::provide_thumbnail_cache();

    // A preset queued from the Presets tab, drained by NewGame on mount.
    let pending_preset = RwSignal::new(Option::<String>::None);
    provide_context(PendingPreset(pending_preset));

    // Bootstrap: try GET /api/v1/me once on mount; if the server
    // recognises our cookie, jump straight to Menu. The 401 case
    // (unauthenticated) leaves us on Landing.
    Effect::new(move |_| {
        // Fire exactly once.
        let already_set = screen.get_untracked() != Screen::Landing;
        if already_set {
            return;
        }
        spawn_local(async move {
            if me_api::get().await.is_ok() {
                screen.set(Screen::Menu);
            }
        });
    });

    // Callbacks reused across screens.
    let go_login = Callback::new(move |_: ()| screen.set(Screen::Login));
    let go_landing = Callback::new(move |_: ()| screen.set(Screen::Landing));
    let go_menu = Callback::new(move |_: ()| screen.set(Screen::Menu));
    let go_newgame = Callback::new(move |_: ()| {
        screen.set(Screen::Menu);
        menu_tab.set(MenuTab::NewGame);
    });
    // Presets tab → load a saved config into the wizard: queue the
    // body and jump to the New-game tab (NewGame applies it on mount).
    let load_preset = Callback::new(move |body_json: String| {
        pending_preset.set(Some(body_json));
        screen.set(Screen::Menu);
        menu_tab.set(MenuTab::NewGame);
    });
    let on_signout = Callback::new(move |_: ()| {
        spawn_local(async move {
            let _ = crate::components::api::auth::signout().await;
            screen.set(Screen::Landing);
        });
    });

    view! {
        <PopupProvider>
        <div class="app" data-density=move || density.get()>
            // ── App bar ─────────────────────────────────────────────────
            <header class="app-bar">
                <div class="brand">
                    <span class="glyph">"⌬"</span>
                    " OPEN4X·VI"
                </div>
                <span class="pill">"HI-FI"</span>
                <nav>
                    {Screen::ALL.iter().copied().map(|s| view! {
                        <button
                            aria-current=move || (screen.get() == s).to_string()
                            on:click=move |_| screen.set(s)
                        >{s.label()}</button>
                    }).collect::<Vec<_>>()}
                </nav>
                <div class="right">
                    <span><span class="dot"></span>" open4x.dev"</span>
                    <span class="kbd">"?"</span>
                </div>
            </header>

            // ── Screen body ─────────────────────────────────────────────
            <div class="screen">
                {move || match screen.get() {
                    Screen::Landing => view! {
                        <Landing on_signin=go_login />
                    }.into_any(),
                    Screen::Login => view! {
                        <Login on_back=go_landing on_authenticated=go_menu />
                    }.into_any(),
                    Screen::Menu => {
                        let on_generated = Callback::new(move |_game_id: String| {
                            menu_tab.set(MenuTab::Ongoing);
                        });
                        view! {
                            <MenuShell tab=menu_tab on_signout=on_signout>
                                {move || match menu_tab.get() {
                                    MenuTab::Ongoing => view! {
                                        <OngoingGames on_new=go_newgame />
                                    }.into_any(),
                                    MenuTab::NewGame => view! {
                                        <NewGame on_generated=on_generated />
                                    }.into_any(),
                                    MenuTab::Profile => view! {
                                        <Profile on_signout=on_signout />
                                    }.into_any(),
                                    MenuTab::Friends => view! { <Friends /> }.into_any(),
                                    MenuTab::Presets => view! {
                                        <Presets on_load=load_preset />
                                    }.into_any(),
                                    MenuTab::Docs => view! { <Docs /> }.into_any(),
                                }}
                            </MenuShell>
                        }.into_any()
                    },
                }}
            </div>
        </div>
        <TweaksPanel density=density />
        </PopupProvider>
    }
}
