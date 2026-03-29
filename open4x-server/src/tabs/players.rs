use leptos::prelude::*;
use crate::types::view::GameView;

#[component]
pub fn PlayersTab(
    game_view: ReadSignal<Option<GameView>>,
) -> impl IntoView {
    let content = move || {
        let Some(gv) = game_view.get() else {
            return view! { <p>"Loading..."</p> }.into_any();
        };

        let my_score = gv.scores.iter()
            .find(|(id, _)| *id == gv.my_civ_id)
            .map(|(_, s)| *s)
            .unwrap_or(0);

        let players: Vec<_> = gv.other_civs.iter().map(|c| {
            (c.name.clone(), c.leader_name.clone(), c.score, format!("{:?}", c.diplomatic_status))
        }).collect();

        view! {
            <div class="tab-content">
                <div class="tech-header">
                    <span class="tech-summary">
                        {format!("{} — Score: {}", gv.my_civ.name, my_score)}
                    </span>
                </div>
                <table class="data-table">
                    <thead>
                        <tr>
                            <th>"Civilization"</th>
                            <th>"Leader"</th>
                            <th>"Score"</th>
                            <th>"Diplomacy"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {players.into_iter().map(|(name, leader, score, diplo)| {
                            let diplo_class = format!("diplo-{}", diplo.to_lowercase());
                            view! {
                                <tr>
                                    <td>{name}</td>
                                    <td>{leader}</td>
                                    <td>{score}</td>
                                    <td class=diplo_class>{diplo}</td>
                                </tr>
                            }
                        }).collect::<Vec<_>>()}
                    </tbody>
                </table>
            </div>
        }.into_any()
    };

    view! { <div>{content}</div> }
}
