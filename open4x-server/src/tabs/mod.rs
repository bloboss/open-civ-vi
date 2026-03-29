pub mod city;
pub mod climate;
pub mod culture;
pub mod data_reports;
pub mod governors;
pub mod great_people;
pub mod players;
pub mod science;

use leptos::prelude::*;
use crate::types::ids::CityId;

/// Which game tab is currently active.
#[derive(Clone, PartialEq, Debug)]
pub enum GameTab {
    /// Default — hex map view with mini info panel.
    Map,
    /// Data tables with sub-tabs (Cities, Resources, Units, MapStats).
    DataReports(DataReportSubTab),
    /// Tech tree.
    Science,
    /// Civic tree.
    Culture,
    /// Governor management (placeholder).
    Governors,
    /// Great People tracking (placeholder).
    GreatPeople,
    /// Climate monitoring (placeholder).
    Climate,
    /// Opponent info.
    Players,
    /// Individual city management.
    City(CityId),
}

#[derive(Clone, PartialEq, Debug)]
pub enum DataReportSubTab {
    Cities,
    Resources,
    Units,
    MapStats,
}

/// Horizontal tab bar displayed below the top bar.
#[component]
pub fn TabBar(
    active_tab: RwSignal<GameTab>,
) -> impl IntoView {
    let tab_btn = move |label: &'static str, tab: GameTab| {
        let tab_clone = tab.clone();
        let is_active = move || {
            let current = active_tab.get();
            std::mem::discriminant(&current) == std::mem::discriminant(&tab_clone)
        };
        let tab_for_click = tab;
        view! {
            <button
                class="tab-btn"
                class:tab-active=is_active
                on:click=move |_| active_tab.set(tab_for_click.clone())
            >
                {label}
            </button>
        }
    };

    view! {
        <div class="tab-bar">
            {tab_btn("Map", GameTab::Map)}
            {tab_btn("Reports", GameTab::DataReports(DataReportSubTab::Cities))}
            {tab_btn("Science", GameTab::Science)}
            {tab_btn("Culture", GameTab::Culture)}
            {tab_btn("Governors", GameTab::Governors)}
            {tab_btn("Great People", GameTab::GreatPeople)}
            {tab_btn("Climate", GameTab::Climate)}
            {tab_btn("Players", GameTab::Players)}
        </div>
    }
}
