use leptos::prelude::*;
use leptos::prelude::LocalStorage;
use crate::types::ids::TechId;
use crate::types::messages::{ClientMessage, GameAction};
use crate::types::view::GameView;
use crate::components::ws::WsClient;

#[component]
pub fn ScienceTab(
    game_view: ReadSignal<Option<GameView>>,
    ws_client: StoredValue<Option<WsClient>, LocalStorage>,
) -> impl IntoView {
    let content = move || {
        let Some(gv) = game_view.get() else {
            return view! { <p>"Loading..."</p> }.into_any();
        };

        let researched = &gv.my_civ.researched_techs;
        let queue = &gv.my_civ.research_queue;
        let science_pt = gv.my_civ.yields.science;

        let current_info = queue.first().map(|tp| {
            let name = gv.tech_tree.nodes.iter()
                .find(|n| n.id == tp.tech_id)
                .map(|n| n.name.clone())
                .unwrap_or_else(|| "?".into());
            let cost = gv.tech_tree.nodes.iter()
                .find(|n| n.id == tp.tech_id)
                .map(|n| n.cost)
                .unwrap_or(0);
            (name, tp.progress, cost, tp.boosted)
        });

        let queued_ids: Vec<TechId> = queue.iter().map(|tp| tp.tech_id).collect();

        // Categorize all tech nodes.
        let nodes: Vec<_> = gv.tech_tree.nodes.iter().map(|n| {
            let status = if researched.contains(&n.id) {
                "researched"
            } else if queued_ids.contains(&n.id) {
                "in-progress"
            } else if n.prerequisites.iter().all(|p| researched.contains(p)) {
                "available"
            } else {
                "locked"
            };
            let progress = queue.iter().find(|tp| tp.tech_id == n.id).map(|tp| tp.progress);
            (n.id, n.name.clone(), n.cost, status, progress, n.eureka_description.clone())
        }).collect();

        view! {
            <div class="tab-content">
                <div class="tech-header">
                    <span class="tech-summary">
                        {format!("Researched: {} | Science/turn: {:+}", researched.len(), science_pt)}
                    </span>
                    {current_info.map(|(name, progress, cost, boosted)| {
                        let boost_tag = if boosted { " [Eureka!]" } else { "" };
                        view! {
                            <div class="tech-current">
                                {format!("Researching: {} [{}/{}]{}", name, progress, cost, boost_tag)}
                                <div class="progress-bar">
                                    <div class="progress-fill" style=move || {
                                        let pct = if cost > 0 { (progress as f64 / cost as f64 * 100.0).min(100.0) } else { 0.0 };
                                        format!("width: {pct:.0}%")
                                    } />
                                </div>
                            </div>
                        }
                    })}
                </div>
                <div class="tech-grid">
                    {nodes.into_iter().map(|(id, name, cost, status, progress, eureka)| {
                        let class = format!("tech-node tech-{status}");
                        let prog_label = progress.map(|p| format!(" [{p}/{cost}]")).unwrap_or_default();
                        view! {
                            <div
                                class=class
                                on:click=move |_| {
                                    if status == "available" {
                                        ws_client.with_value(|ws| {
                                            if let Some(ws) = ws {
                                                ws.send(&ClientMessage::Action(GameAction::QueueResearch { tech: id }));
                                            }
                                        });
                                    }
                                }
                            >
                                <div class="tech-name">{format!("{name}{prog_label}")}</div>
                                <div class="tech-cost">{format!("Cost: {cost}")}</div>
                                {(!eureka.is_empty()).then(|| view! {
                                    <div class="tech-eureka">{eureka}</div>
                                })}
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </div>
        }.into_any()
    };

    view! { <div>{content}</div> }
}
