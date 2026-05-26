//! Tabbar — 9-tab nav, end-turn group, screen-stub primitive.
//!
//! Pixel-mirrors `Open4X.html` lines 76-98 (tabbar) and lines 177-232
//! (the per-screen "Coming next" placeholder). Tab labels, badges, and
//! the End-Turn meta dot all follow `open4x.js::bootTabs` / the
//! end-turn helpers.

use leptos::prelude::*;
use open4x_protocol::v1::web::city_data::CityData;
use open4x_protocol::v1::web::civics_tree::CivicsTreeView;
use open4x_protocol::v1::web::tech_tree::TechTreeView;
use open4x_protocol::v1::web::turn_queue::TurnQueue;
use open4x_protocol::v1::web::unit_data::UnitData;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Tab {
    Hud,
    City,
    Units,
    Tech,
    Civics,
    Diplomacy,
    Government,
    Empire,
    Victory,
}

impl Tab {
    pub const ALL: &'static [Tab] = &[
        Tab::Hud, Tab::City, Tab::Units, Tab::Tech, Tab::Civics,
        Tab::Diplomacy, Tab::Government, Tab::Empire, Tab::Victory,
    ];

    /// Visible label.
    pub fn label(self) -> &'static str {
        match self {
            Tab::Hud => "Map",
            Tab::City => "Cities",
            Tab::Units => "Units",
            Tab::Tech => "Tech",
            Tab::Civics => "Civics",
            Tab::Diplomacy => "Diplomacy",
            Tab::Government => "Government",
            Tab::Empire => "Empire",
            Tab::Victory => "Victory",
        }
    }

    /// `data-screen=` attribute key — matches Open4X.html's identifiers.
    pub fn key(self) -> &'static str {
        match self {
            Tab::Hud => "hud",
            Tab::City => "city",
            Tab::Units => "units",
            Tab::Tech => "tech",
            Tab::Civics => "civics",
            Tab::Diplomacy => "dipl",
            Tab::Government => "govt",
            Tab::Empire => "overview",
            Tab::Victory => "victory",
        }
    }

    /// 1-based keyboard shortcut shown in the tab key chip.
    pub fn shortcut(self) -> u8 {
        Tab::ALL.iter().position(|t| *t == self).unwrap_or(0) as u8 + 1
    }

    /// Try to parse a single ASCII digit ('1'..='9') as a tab.
    pub fn from_digit(d: char) -> Option<Tab> {
        let i = d.to_digit(10)? as usize;
        if i == 0 { return None; }
        Tab::ALL.get(i - 1).copied()
    }
}

#[component]
pub fn Tabbar(
    active: RwSignal<Tab>,
    /// Used for the city-count badge.
    cities: LocalResource<Option<CityData>>,
    /// Used for the unit-count badge.
    units: LocalResource<Option<UnitData>>,
    /// Used for the active-research badge.
    tech: LocalResource<Option<TechTreeView>>,
    /// Used for the active-civic badge.
    civics: LocalResource<Option<CivicsTreeView>>,
    /// Used for the end-turn meta dot + button label.
    turn_queue: LocalResource<Option<TurnQueue>>,
    /// Click handler for the End Turn / Resolve required button.
    on_end_turn: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="tabbar">
            <nav class="tabs">
                {Tab::ALL.iter().copied().map(|t| view! {
                    <TabButton tab=t active=active
                               cities=cities units=units tech=tech civics=civics />
                }).collect::<Vec<_>>()}
            </nav>
            <EndTurnGroup turn_queue=turn_queue on_end_turn=on_end_turn />
        </div>
    }
}

#[component]
fn TabButton(
    tab: Tab,
    active: RwSignal<Tab>,
    cities: LocalResource<Option<CityData>>,
    units: LocalResource<Option<UnitData>>,
    tech: LocalResource<Option<TechTreeView>>,
    civics: LocalResource<Option<CivicsTreeView>>,
) -> impl IntoView {
    let label = tab.label();
    let key   = tab.shortcut();

    let badge = move || -> AnyView {
        match tab {
            Tab::City => count_badge(cities.get().as_deref().and_then(|w| w.as_ref().map(|c| c.cities.len()))).into_any(),
            Tab::Units => count_badge(units.get().as_deref().and_then(|w| w.as_ref().map(|u| u.units.len()))).into_any(),
            Tab::Tech => research_badge(tech.get().as_deref().and_then(|w| w.as_ref().map(|t| {
                t.techs.iter().find(|n| n.status == "current").map(|n| (n.name.clone(), n.progress.unwrap_or(0), n.cost))
            }).flatten()), "attn").into_any(),
            Tab::Civics => research_badge(civics.get().as_deref().and_then(|w| w.as_ref().map(|c| {
                c.civics.iter().find(|n| n.status == "current").map(|n| (n.name.clone(), n.progress.unwrap_or(0), n.cost))
            }).flatten()), "").into_any(),
            _ => ().into_any(),
        }
    };

    view! {
        <button
            class="tab"
            class:active=move || active.get() == tab
            attr:data-screen=tab.key()
            on:click=move |_| active.set(tab)
        >
            <span>{label}</span>
            <span class="tab-key">{key}</span>
            {badge}
        </button>
    }
}

fn count_badge(n: Option<usize>) -> AnyView {
    let txt = match n { Some(v) => v.to_string(), None => "—".to_string() };
    view! { <span class="tab-badge">{txt}</span> }.into_any()
}

fn research_badge(
    cur: Option<(String, u32, u32)>,
    extra_cls: &'static str,
) -> AnyView {
    let Some((name, prog, cost)) = cur else { return ().into_any() };
    let pct = if cost == 0 { 0 } else { ((prog as f64 / cost as f64) * 100.0).round() as i32 };
    let cls = if extra_cls.is_empty() { "tab-badge".to_string() } else { format!("tab-badge {extra_cls}") };
    view! {
        <span class=cls title=format!("Researching {name} ({prog}/{cost})")>
            {format!("{pct}%")}
        </span>
    }.into_any()
}

#[component]
pub fn EndTurnGroup(
    turn_queue: LocalResource<Option<TurnQueue>>,
    on_end_turn: Callback<()>,
) -> impl IntoView {
    let stats = move || {
        let wrap = turn_queue.get();
        let Some(wrap) = wrap.as_deref() else { return (0usize, 0usize) };
        let Some(q) = wrap.as_ref() else { return (0usize, 0usize) };
        let total = q.items.len();
        let required = q.items.iter().filter(|i| i.required).count();
        (total, required)
    };

    let summary = move || {
        let (total, required) = stats();
        if total == 0 { return "no actions pending".to_string(); }
        if required > 0 {
            format!("{required} required action{}",
                if required == 1 { "" } else { "s" })
        } else {
            format!("{total} optional pending")
        }
    };

    let dot_ok    = move || stats().1 == 0;
    let btn_label = move || if stats().1 > 0 { "Resolve required" } else { "End Turn" };
    let disabled  = move || stats().1 > 0;

    view! {
        <div class="end-turn-group">
            <div class="end-turn-meta">
                <span class="dot" class:ok=dot_ok></span>
                <span>{summary}</span>
            </div>
            <button
                class="btn-primary"
                class:disabled=disabled
                on:click=move |_| on_end_turn.run(())
            >
                <span>{btn_label}</span>
                <span class="kbd">"\u{21B5}"</span>
            </button>
        </div>
    }
}

/// Placeholder body for the 8 screens that aren't yet ported. Matches
/// the design's own "Coming next" treatment in Open4X.html lines 177-232.
#[component]
pub fn ScreenStub(
    #[prop(into)] title: String,
    #[prop(into)] desc: String,
) -> impl IntoView {
    view! {
        <div class="screen-stub">
            <div class="screen-stub-card">
                <span class="kicker">"Coming next"</span>
                <h2>{title}</h2>
                <p>{desc}</p>
            </div>
        </div>
    }
}
