use leptos::prelude::*;
use leptos::prelude::LocalStorage;
use crate::types::ids::*;
use crate::types::view::*;
use crate::types::enums::ProductionItemView;
use crate::types::messages::{ClientMessage, GameAction};
use crate::components::ws::WsClient;
use super::GameTab;

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[component]
pub fn CityTab(
    city_id: CityId,
    game_view: ReadSignal<Option<GameView>>,
    ws_client: StoredValue<Option<WsClient>, LocalStorage>,
    active_tab: RwSignal<GameTab>,
) -> impl IntoView {
    let content = move || {
        let Some(gv) = game_view.get() else {
            return view! { <p>"Loading..."</p> }.into_any();
        };

        let Some(city) = gv.cities.iter().find(|c| c.id == city_id) else {
            return view! { <p>"City not found."</p> }.into_any();
        };

        let city_name = city.name.clone();
        let population = city.population;
        let food_stored = city.food_stored;
        let food_to_grow = city.food_to_grow;
        let prod_stored = city.production_stored;
        let is_capital = city.is_capital;
        let is_own = city.is_own;
        let worked_count = city.worked_tiles.len();

        let prod_info: Option<(String, u32)> = city.production_queue.first()
            .and_then(|item| match item {
                ProductionItemView::Unit(tid) => gv.unit_type_defs.iter()
                    .find(|d| d.id == *tid)
                    .map(|d| (capitalize(&d.name), d.production_cost)),
                ProductionItemView::Building(bid) => gv.building_defs.iter()
                    .find(|d| d.id == *bid)
                    .map(|d| (d.name.clone(), d.cost)),
                _ => None,
            });

        let prod_label = prod_info.as_ref()
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| "Idle".into());
        let prod_cost_str = prod_info.as_ref()
            .map(|(_, cost)| format!("{prod_stored}/{cost}"))
            .unwrap_or_else(|| format!("{prod_stored}/—"));

        // Resolve building IDs to names
        let building_names: Vec<String> = city.buildings.iter()
            .map(|bid| {
                gv.building_defs.iter()
                    .find(|d| d.id == *bid)
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|| format!("{:?}", bid))
            })
            .collect();

        let unit_defs: Vec<(UnitTypeId, String, u32)> = gv.unit_type_defs.iter()
            .map(|d| (d.id, capitalize(&d.name), d.production_cost))
            .collect();

        let bldg_defs: Vec<(BuildingId, String, u32)> = gv.building_defs.iter()
            .map(|d| (d.id, d.name.clone(), d.cost))
            .collect();

        let capital_tag = if is_capital { " ★" } else { "" };
        let cid = city_id;

        view! {
            <div class="tab-content city-tab">
                <div class="city-header">
                    <button class="btn btn-ghost" on:click=move |_| active_tab.set(GameTab::Map)>"← Back to Map"</button>
                    <h2>{format!("{city_name}{capital_tag}")}</h2>
                </div>

                <div class="city-stats">
                    <div class="stat-card">
                        <div class="stat-label">"Population"</div>
                        <div class="stat-value">{population}</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-label">"Food"</div>
                        <div class="stat-value">{format!("{food_stored}/{food_to_grow}")}</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-label">"Production"</div>
                        <div class="stat-value">{format!("{prod_label} [{prod_cost_str}]")}</div>
                    </div>
                    <div class="stat-card">
                        <div class="stat-label">"Worked Tiles"</div>
                        <div class="stat-value">{format!("{worked_count}/{population}")}</div>
                    </div>
                </div>

                <div class="city-section">
                    <h3>"Buildings"</h3>
                    {if building_names.is_empty() {
                        view! { <p class="empty-note">"No buildings built."</p> }.into_any()
                    } else {
                        view! {
                            <ul class="item-list">
                                {building_names.into_iter().map(|name| {
                                    view! { <li>{name}</li> }
                                }).collect::<Vec<_>>()}
                            </ul>
                        }.into_any()
                    }}
                </div>

                {is_own.then(|| {
                    view! {
                        <div class="city-section">
                            <h3>"Production Queue"</h3>

                            <h4>"Units"</h4>
                            <div class="production-options">
                                {unit_defs.into_iter().map(|(type_id, name, cost)| {
                                    view! {
                                        <button
                                            class="btn btn-ghost production-btn"
                                            on:click=move |_| {
                                                ws_client.with_value(|ws| {
                                                    if let Some(ws) = ws {
                                                        ws.send(&ClientMessage::Action(GameAction::QueueProduction {
                                                            city: cid,
                                                            item: ProductionItemView::Unit(type_id),
                                                        }));
                                                    }
                                                });
                                            }
                                        >
                                            {format!("{name} ({cost})")}
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>

                            <h4>"Buildings"</h4>
                            <div class="production-options">
                                {bldg_defs.into_iter().map(|(bid, name, cost)| {
                                    view! {
                                        <button
                                            class="btn btn-ghost production-btn"
                                            on:click=move |_| {
                                                ws_client.with_value(|ws| {
                                                    if let Some(ws) = ws {
                                                        ws.send(&ClientMessage::Action(GameAction::QueueProduction {
                                                            city: cid,
                                                            item: ProductionItemView::Building(bid),
                                                        }));
                                                    }
                                                });
                                            }
                                        >
                                            {format!("{name} ({cost})")}
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>

                            <button
                                class="btn btn-ghost"
                                style="margin-top:8px;width:100%"
                                on:click=move |_| {
                                    ws_client.with_value(|ws| {
                                        if let Some(ws) = ws {
                                            ws.send(&ClientMessage::Action(GameAction::CancelProduction {
                                                city: cid,
                                                index: 0,
                                            }));
                                        }
                                    });
                                }
                            >
                                "Cancel Current Production"
                            </button>
                        </div>
                    }
                })}
            </div>
        }.into_any()
    };

    view! { <div>{content}</div> }
}
