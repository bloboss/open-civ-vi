//! `<Tag>` — small inline tag with optional `variant` styling.

use leptos::prelude::*;

#[component]
pub fn Tag(#[prop(optional, into)] variant: &'static str, children: Children) -> impl IntoView {
    let class = format!("tag {variant}");
    view! { <span class=class>{children()}</span> }
}
