mod auth;
mod pages;
pub mod session;
mod hexmap;
mod ws;

use leptos::prelude::*;
use pages::{HomePage, SettingsPage, PlayersPage, MapConfigPage};
use crate::pages::game::GamePage;
use crate::session::GameConfig;

/// Top-level page discriminant.
#[derive(Clone, PartialEq, Debug)]
pub enum Page {
    Home,
    Settings,
    Players,
    MapConfig,
    Game,
}

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    let page        = RwSignal::new(Page::Home);
    let game_config = RwSignal::new(GameConfig::default());

    view! {
        <Show when=move || page.get() == Page::Home>
            <HomePage
                on_new_game=move || page.set(Page::MapConfig)
                on_settings=move || page.set(Page::Settings)
                on_players=move || page.set(Page::Players)
            />
        </Show>
        <Show when=move || page.get() == Page::Settings>
            <SettingsPage on_back=move || page.set(Page::Home) />
        </Show>
        <Show when=move || page.get() == Page::Players>
            <PlayersPage on_back=move || page.set(Page::Home) />
        </Show>
        <Show when=move || page.get() == Page::MapConfig>
            <MapConfigPage
                on_start=move |cfg| { game_config.set(cfg); page.set(Page::Game); }
                on_back=move || page.set(Page::Home)
            />
        </Show>
        <Show when=move || page.get() == Page::Game>
            <GamePage
                game_config=game_config
                on_quit=move || page.set(Page::Home)
            />
        </Show>
    }
}

/// WASM entry point called by trunk.
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
