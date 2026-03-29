use leptos::prelude::*;

#[component]
pub fn ClimateTab() -> impl IntoView {
    view! {
        <div class="tab-content placeholder-tab">
            <h2>"Climate"</h2>
            <p class="placeholder-msg">"Climate monitoring coming soon."</p>
            <p class="placeholder-hint">"Track global CO2 levels, sea level rise, and natural disaster frequency."</p>
        </div>
    }
}
