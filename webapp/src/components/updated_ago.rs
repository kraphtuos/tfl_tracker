use crate::utils::{format_age, now_ms};
use gloo_timers::callback::Interval;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct UpdatedAgoProps {
    /// Time of the update in milliseconds since the epoch
    pub at: f64,
}

/// "Updated 12s ago" label that re-renders itself every second.
#[function_component(UpdatedAgo)]
pub fn updated_ago(props: &UpdatedAgoProps) -> Html {
    let now = use_state(now_ms);
    {
        let now = now.clone();
        use_effect_with((), move |_| {
            let interval = Interval::new(1_000, move || now.set(now_ms()));
            move || drop(interval)
        });
    }

    html! {
        <span class="text-xs text-gray-500 text-center sm:text-right" aria-live="off">
            { format!("Updated {}", format_age(*now - props.at)) }
        </span>
    }
}
