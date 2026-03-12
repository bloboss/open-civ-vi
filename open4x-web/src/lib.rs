mod pages;
mod session;
mod hexmap;

use leptos::prelude::*;
use pages::{HomePage, SettingsPage, PlayersPage};
use crate::pages::game::GamePage;

/// Top-level page discriminant.
#[derive(Clone, PartialEq, Debug)]
pub enum Page {
    Home,
    Settings,
    Players,
    Game,
}

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    let page = RwSignal::new(Page::Home);

    view! {
        <Show when=move || page.get() == Page::Home>
            <HomePage
                on_new_game=move || page.set(Page::Game)
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
        <Show when=move || page.get() == Page::Game>
            <GamePage on_quit=move || page.set(Page::Home) />
        </Show>
    }
}

/// WASM entry point called by trunk.
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
