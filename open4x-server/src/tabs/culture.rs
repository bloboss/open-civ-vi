use leptos::prelude::*;
use leptos::prelude::LocalStorage;
use crate::types::messages::{ClientMessage, GameAction};
use crate::types::view::GameView;
use crate::components::ws::WsClient;

#[component]
pub fn CultureTab(
    game_view: ReadSignal<Option<GameView>>,
    ws_client: StoredValue<Option<WsClient>, LocalStorage>,
) -> impl IntoView {
    let content = move || {
        let Some(gv) = game_view.get() else {
            return view! { <p>"Loading..."</p> }.into_any();
        };

        let completed = &gv.my_civ.completed_civics;
        let in_progress = &gv.my_civ.civic_in_progress;
        let culture_pt = gv.my_civ.yields.culture;
        let government = gv.my_civ.current_government.clone();

        let current_info = in_progress.as_ref().map(|cp| {
            let name = gv.civic_tree.nodes.iter()
                .find(|n| n.id == cp.civic_id)
                .map(|n| n.name.clone())
                .unwrap_or_else(|| "?".into());
            let cost = gv.civic_tree.nodes.iter()
                .find(|n| n.id == cp.civic_id)
                .map(|n| n.cost)
                .unwrap_or(0);
            (name, cp.progress, cost, cp.inspired)
        });

        let in_progress_id = in_progress.as_ref().map(|cp| cp.civic_id);

        let nodes: Vec<_> = gv.civic_tree.nodes.iter().map(|n| {
            let status = if completed.contains(&n.id) {
                "researched"
            } else if in_progress_id == Some(n.id) {
                "in-progress"
            } else if n.prerequisites.iter().all(|p| completed.contains(p)) {
                "available"
            } else {
                "locked"
            };
            let progress = in_progress.as_ref()
                .filter(|cp| cp.civic_id == n.id)
                .map(|cp| cp.progress);
            (n.id, n.name.clone(), n.cost, status, progress, n.inspiration_description.clone())
        }).collect();

        let gov_label = government.unwrap_or_else(|| "None".into());

        view! {
            <div class="tab-content">
                <div class="tech-header">
                    <span class="tech-summary">
                        {format!("Civics: {} | Culture/turn: {:+} | Government: {}", completed.len(), culture_pt, gov_label)}
                    </span>
                    {current_info.map(|(name, progress, cost, inspired)| {
                        let inspire_tag = if inspired { " [Inspired!]" } else { "" };
                        view! {
                            <div class="tech-current">
                                {format!("Developing: {} [{}/{}]{}", name, progress, cost, inspire_tag)}
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
                    {nodes.into_iter().map(|(id, name, cost, status, progress, inspiration)| {
                        let class = format!("tech-node tech-{status}");
                        let prog_label = progress.map(|p| format!(" [{p}/{cost}]")).unwrap_or_default();
                        view! {
                            <div
                                class=class
                                on:click=move |_| {
                                    if status == "available" {
                                        ws_client.with_value(|ws| {
                                            if let Some(ws) = ws {
                                                ws.send(&ClientMessage::Action(GameAction::QueueCivic { civic: id }));
                                            }
                                        });
                                    }
                                }
                            >
                                <div class="tech-name">{format!("{name}{prog_label}")}</div>
                                <div class="tech-cost">{format!("Cost: {cost}")}</div>
                                {(!inspiration.is_empty()).then(|| view! {
                                    <div class="tech-eureka">{inspiration}</div>
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
