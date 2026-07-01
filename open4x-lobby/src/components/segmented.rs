//! `<Segmented>` — segmented control rendered as `.seg > button[aria-pressed]`.

use leptos::prelude::*;

#[derive(Clone)]
pub struct Segment {
    pub value: String,
    pub label: String,
}

impl Segment {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        Self::new(s, s)
    }
}

#[component]
pub fn Segmented(
    #[prop(into)] options: Signal<Vec<Segment>>,
    #[prop(into)] value: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="seg">
            {move || {
                options.get().into_iter().map(|seg| {
                    let v = seg.value.clone();
                    let v_for_click = v.clone();
                    let v_for_attr = v.clone();
                    view! {
                        <button
                            aria-pressed=move || (value.get() == v_for_attr).to_string()
                            on:click=move |_| value.set(v_for_click.clone())
                        >
                            {seg.label.clone()}
                        </button>
                    }
                }).collect::<Vec<_>>()
            }}
        </div>
    }
}
