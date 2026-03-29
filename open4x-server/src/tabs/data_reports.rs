use leptos::prelude::*;
use crate::types::enums::*;
use crate::types::view::GameView;
use super::{GameTab, DataReportSubTab};

#[component]
pub fn DataReportsTab(
    game_view: ReadSignal<Option<GameView>>,
    active_tab: RwSignal<GameTab>,
) -> impl IntoView {
    let current_sub = move || {
        match active_tab.get() {
            GameTab::DataReports(st) => st,
            _ => DataReportSubTab::Cities,
        }
    };

    let sub_btn = move |label: &'static str, st: DataReportSubTab| {
        let st_clone = st.clone();
        let is_active = move || current_sub() == st_clone;
        let st_for_click = st;
        view! {
            <button
                class="tab-btn sub-tab-btn"
                class:tab-active=is_active
                on:click=move |_| active_tab.set(GameTab::DataReports(st_for_click.clone()))
            >
                {label}
            </button>
        }
    };

    view! {
        <div class="tab-content">
            <div class="sub-tab-bar">
                {sub_btn("Cities", DataReportSubTab::Cities)}
                {sub_btn("Resources", DataReportSubTab::Resources)}
                {sub_btn("Units", DataReportSubTab::Units)}
                {sub_btn("Map Stats", DataReportSubTab::MapStats)}
            </div>
            <div class="tab-body">
                {move || {
                    match current_sub() {
                        DataReportSubTab::Cities => view! { <CitiesTable game_view=game_view /> }.into_any(),
                        DataReportSubTab::Resources => view! { <ResourcesTable game_view=game_view /> }.into_any(),
                        DataReportSubTab::Units => view! { <UnitsTable game_view=game_view /> }.into_any(),
                        DataReportSubTab::MapStats => view! { <MapStatsPanel game_view=game_view /> }.into_any(),
                    }
                }}
            </div>
        </div>
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[component]
fn CitiesTable(game_view: ReadSignal<Option<GameView>>) -> impl IntoView {
    let rows = move || {
        let Some(gv) = game_view.get() else { return Vec::new() };
        gv.cities.iter().filter(|c| c.is_own).map(|c| {
            let prod_name = c.production_queue.first().map(|item| match item {
                ProductionItemView::Unit(tid) => gv.unit_type_defs.iter()
                    .find(|d| d.id == *tid)
                    .map(|d| capitalize(&d.name))
                    .unwrap_or_else(|| "Unit".into()),
                ProductionItemView::Building(bid) => gv.building_defs.iter()
                    .find(|d| d.id == *bid)
                    .map(|d| capitalize(&d.name))
                    .unwrap_or_else(|| "Building".into()),
                ProductionItemView::District(d) => format!("{d:?}"),
                ProductionItemView::Wonder(_) => "Wonder".into(),
            }).unwrap_or_else(|| "Idle".into());
            let capital = if c.is_capital { " *" } else { "" };
            view! {
                <tr>
                    <td>{format!("{}{}", c.name, capital)}</td>
                    <td>{c.population}</td>
                    <td>{format!("{}/{}", c.food_stored, c.food_to_grow)}</td>
                    <td>{c.production_stored}</td>
                    <td>{prod_name}</td>
                    <td>{c.buildings.len()}</td>
                    <td>{c.worked_tiles.len()}</td>
                </tr>
            }
        }).collect::<Vec<_>>()
    };

    view! {
        <table class="data-table">
            <thead>
                <tr>
                    <th>"City"</th>
                    <th>"Pop"</th>
                    <th>"Food"</th>
                    <th>"Prod"</th>
                    <th>"Building"</th>
                    <th>"Buildings"</th>
                    <th>"Worked"</th>
                </tr>
            </thead>
            <tbody>{rows}</tbody>
        </table>
    }
}

#[component]
fn ResourcesTable(game_view: ReadSignal<Option<GameView>>) -> impl IntoView {
    let rows = move || {
        let Some(gv) = game_view.get() else { return Vec::new() };
        let mut counts: std::collections::HashMap<String, (u32, u32)> = std::collections::HashMap::new();
        for tile in &gv.board.tiles {
            if let Some(res) = &tile.resource {
                let name = format!("{res:?}");
                let entry = counts.entry(name).or_insert((0, 0));
                entry.0 += 1;
                if tile.improvement.is_some() {
                    entry.1 += 1;
                }
            }
        }
        let mut sorted: Vec<_> = counts.into_iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        sorted.into_iter().map(|(name, (total, improved))| {
            view! {
                <tr>
                    <td>{name}</td>
                    <td>{total}</td>
                    <td>{improved}</td>
                </tr>
            }
        }).collect::<Vec<_>>()
    };

    view! {
        <table class="data-table">
            <thead><tr><th>"Resource"</th><th>"Count"</th><th>"Improved"</th></tr></thead>
            <tbody>{rows}</tbody>
        </table>
    }
}

#[component]
fn UnitsTable(game_view: ReadSignal<Option<GameView>>) -> impl IntoView {
    let rows = move || {
        let Some(gv) = game_view.get() else { return Vec::new() };
        gv.units.iter().filter(|u| u.is_own).map(|u| {
            let type_name = gv.unit_type_defs.iter()
                .find(|d| d.id == u.unit_type)
                .map(|d| capitalize(&d.name))
                .unwrap_or_else(|| "Unknown".into());
            view! {
                <tr>
                    <td>{type_name}</td>
                    <td>{format!("({}, {})", u.coord.q, u.coord.r)}</td>
                    <td>{u.health}</td>
                    <td>{format!("{}/{}", u.movement_left, u.max_movement)}</td>
                    <td>{u.combat_strength.map(|s| format!("{s}")).unwrap_or_else(|| "-".into())}</td>
                </tr>
            }
        }).collect::<Vec<_>>()
    };

    view! {
        <table class="data-table">
            <thead>
                <tr>
                    <th>"Type"</th>
                    <th>"Location"</th>
                    <th>"HP"</th>
                    <th>"Move"</th>
                    <th>"Combat"</th>
                </tr>
            </thead>
            <tbody>{rows}</tbody>
        </table>
    }
}

#[component]
fn MapStatsPanel(game_view: ReadSignal<Option<GameView>>) -> impl IntoView {
    let stats = move || {
        let Some(gv) = game_view.get() else { return Vec::new() };
        let mut terrain: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        let mut features: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        let mut resources: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        let mut visible = 0u32;

        for tile in &gv.board.tiles {
            *terrain.entry(format!("{:?}", tile.terrain)).or_default() += 1;
            if let Some(f) = &tile.feature {
                *features.entry(format!("{f:?}")).or_default() += 1;
            }
            if let Some(r) = &tile.resource {
                *resources.entry(format!("{r:?}")).or_default() += 1;
            }
            if tile.visibility == TileVisibility::Visible {
                visible += 1;
            }
        }

        let enemy_cities = gv.cities.iter().filter(|c| !c.is_own).count();
        let enemy_units = gv.units.iter().filter(|u| !u.is_own).count();

        let mut rows: Vec<AnyView> = Vec::new();
        rows.push(view! {
            <tr class="data-section-header"><td colspan="2">"Map Overview"</td></tr>
        }.into_any());
        rows.push(view! {
            <tr><td>"Tiles explored"</td><td>{gv.board.tiles.len()}</td></tr>
        }.into_any());
        rows.push(view! {
            <tr><td>"Tiles visible"</td><td>{visible}</td></tr>
        }.into_any());
        rows.push(view! {
            <tr><td>"Enemy cities visible"</td><td>{enemy_cities}</td></tr>
        }.into_any());
        rows.push(view! {
            <tr><td>"Enemy units visible"</td><td>{enemy_units}</td></tr>
        }.into_any());

        rows.push(view! {
            <tr class="data-section-header"><td colspan="2">"Terrain"</td></tr>
        }.into_any());
        let mut terrain_sorted: Vec<_> = terrain.into_iter().collect();
        terrain_sorted.sort_by(|a, b| b.1.cmp(&a.1));
        for (name, count) in terrain_sorted {
            rows.push(view! {
                <tr><td>{name}</td><td>{count}</td></tr>
            }.into_any());
        }

        if !features.is_empty() {
            rows.push(view! {
                <tr class="data-section-header"><td colspan="2">"Features"</td></tr>
            }.into_any());
            let mut features_sorted: Vec<_> = features.into_iter().collect();
            features_sorted.sort_by(|a, b| b.1.cmp(&a.1));
            for (name, count) in features_sorted {
                rows.push(view! {
                    <tr><td>{name}</td><td>{count}</td></tr>
                }.into_any());
            }
        }

        if !resources.is_empty() {
            rows.push(view! {
                <tr class="data-section-header"><td colspan="2">"Resources"</td></tr>
            }.into_any());
            let mut resources_sorted: Vec<_> = resources.into_iter().collect();
            resources_sorted.sort_by(|a, b| a.0.cmp(&b.0));
            for (name, count) in resources_sorted {
                rows.push(view! {
                    <tr><td>{name}</td><td>{count}</td></tr>
                }.into_any());
            }
        }

        rows
    };

    view! {
        <table class="data-table">
            <tbody>{stats}</tbody>
        </table>
    }
}
