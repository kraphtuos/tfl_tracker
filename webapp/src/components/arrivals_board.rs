use crate::api::{Arrival, fetch_arrivals};
use crate::arrivals::group_arrivals_by_line_platform;
use crate::components::spinner::LoadingSpinner;
use crate::components::updated_ago::UpdatedAgo;
use crate::stations::Station;
use crate::utils::{format_arrival_time, line_theme, mode_button_classes, now_ms, page_visible};
use gloo_events::EventListener;
use gloo_timers::callback::Interval;
use std::collections::HashSet;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

/// Number of arrivals shown per platform before "Show all"
const COLLAPSED_ARRIVALS: usize = 3;

/// How often arrivals refresh while the page is visible
const REFRESH_INTERVAL_MS: u32 = 30_000;

#[derive(Properties, PartialEq)]
pub struct ArrivalsBoardProps {
    /// The board fetches arrivals for this station on mount; key it by station ID
    pub station: Station,
    /// IDs of arrivals that are already being tracked
    pub tracked_ids: Rc<HashSet<String>>,
    pub on_track: Callback<Arrival>,
    pub collapsed_lines: Rc<HashSet<String>>,
    pub on_toggle_line: Callback<String>,
}

#[function_component(ArrivalsBoard)]
pub fn arrivals_board(props: &ArrivalsBoardProps) -> Html {
    // None until the first successful load
    let arrivals = use_state(|| None::<Rc<Vec<Arrival>>>);
    let error = use_state(|| None::<String>);
    let fetching = use_state(|| false);
    // Time of the last successful load, for display and for the timers below
    let updated_at = use_state(|| None::<f64>);
    let last_loaded = use_mut_ref(|| 0.0);
    let in_flight = use_mut_ref(|| false);
    let expanded_platforms = use_state(HashSet::<String>::new);
    let show_stop_ids = use_state(|| false);

    let load = {
        let arrivals = arrivals.clone();
        let error = error.clone();
        let fetching = fetching.clone();
        let updated_at = updated_at.clone();
        let last_loaded = last_loaded.clone();
        let stop_ids = props.station.stop_ids();
        Callback::from(move |()| {
            if in_flight.replace(true) {
                return;
            }
            fetching.set(true);
            let arrivals = arrivals.clone();
            let error = error.clone();
            let fetching = fetching.clone();
            let updated_at = updated_at.clone();
            let last_loaded = last_loaded.clone();
            let in_flight = in_flight.clone();
            let stop_ids = stop_ids.clone();
            spawn_local(async move {
                match fetch_arrivals(&stop_ids).await {
                    Ok(data) => {
                        let now = now_ms();
                        *last_loaded.borrow_mut() = now;
                        updated_at.set(Some(now));
                        arrivals.set(Some(Rc::new(data.arrivals)));
                        error.set(
                            data.partial_error
                                .map(|e| format!("Some arrivals couldn't be loaded. {e}")),
                        );
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
                *in_flight.borrow_mut() = false;
                fetching.set(false);
            });
        })
    };

    // Load on mount, then refresh periodically while the page is visible and
    // straight away when returning to a page whose data has gone stale
    {
        let load = load.clone();
        use_effect_with((), move |_| {
            load.emit(());
            let refresh_if_stale = move || {
                let age = now_ms() - *last_loaded.borrow();
                if page_visible() && age >= f64::from(REFRESH_INTERVAL_MS) - 1_000.0 {
                    load.emit(());
                }
            };
            let interval = Interval::new(REFRESH_INTERVAL_MS, {
                let refresh_if_stale = refresh_if_stale.clone();
                move || refresh_if_stale()
            });
            let visibility = web_sys::window()
                .and_then(|w| w.document())
                .map(|document| {
                    EventListener::new(&document, "visibilitychange", move |_| refresh_if_stale())
                });
            move || drop((interval, visibility))
        });
    }

    let grouped = use_memo((*arrivals).clone(), |arrivals| {
        arrivals
            .as_deref()
            .map(|a| group_arrivals_by_line_platform(a))
            .unwrap_or_default()
    });

    let toggle_platform = {
        let expanded_platforms = expanded_platforms.clone();
        Callback::from(move |platform_id: String| {
            let mut expanded = (*expanded_platforms).clone();
            if !expanded.remove(&platform_id) {
                expanded.insert(platform_id);
            }
            expanded_platforms.set(expanded);
        })
    };

    let toggle_stop_ids = {
        let show_stop_ids = show_stop_ids.clone();
        Callback::from(move |_: MouseEvent| show_stop_ids.set(!*show_stop_ids))
    };

    let station = &props.station;

    let header = html! {
        <div class="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-3 mb-4">
            <div>
                <h2 class="text-lg sm:text-xl font-semibold">{ &station.name }</h2>
                <div class="text-gray-600 text-sm">
                    <button
                        onclick={toggle_stop_ids}
                        aria-expanded={show_stop_ids.to_string()}
                        class="text-blue-600 hover:text-blue-800 text-sm py-1"
                    >
                        if *show_stop_ids {
                            { "Hide Station IDs" }
                        } else {
                            { format!("Show Station IDs ({})", station.stops.len()) }
                        }
                    </button>
                    if *show_stop_ids {
                        <div class="mt-2 ml-2 text-sm space-y-1">
                            { for station.stops.iter().map(|stop| html! {
                                <div>{ format!("{} ({})", stop.id, stop.modes.join(", ")) }</div>
                            }) }
                        </div>
                    }
                </div>
            </div>
            <div class="flex flex-col items-stretch sm:items-end gap-1 w-full sm:w-auto">
            <button
                onclick={load.reform(|_: MouseEvent| ())}
                class="w-full sm:w-auto bg-blue-500 hover:bg-blue-700 disabled:opacity-75 text-white font-bold py-2 px-4 rounded"
                disabled={*fetching}
            >
                if *fetching {
                    <span class="flex items-center justify-center">
                        <LoadingSpinner />
                        <span class="ml-2">{ "Updating..." }</span>
                    </span>
                } else {
                    { "Refresh Arrivals" }
                }
            </button>
            if let Some(at) = *updated_at {
                <UpdatedAgo {at} />
            }
            </div>
        </div>
    };

    let body = match &*arrivals {
        None if *fetching => html! {
            <div class="flex items-center justify-center p-4 sm:p-6">
                <LoadingSpinner />
                <p class="ml-2">{ "Fetching arrival information..." }</p>
            </div>
        },
        None => html! {},
        Some(arrivals) if arrivals.is_empty() => html! {
            <div class="text-center p-4 sm:p-6 text-gray-500">
                { "No arrivals are currently predicted for this station." }
            </div>
        },
        Some(_) => html! {
            <div class="space-y-4 sm:space-y-6">
                { for grouped.iter().map(|(line_name, platforms)| {
                    let theme = line_theme(line_name);
                    let is_collapsed = props.collapsed_lines.contains(line_name);
                    let on_toggle = {
                        let line_name = line_name.clone();
                        props.on_toggle_line.reform(move |_: MouseEvent| line_name.clone())
                    };

                    html! {
                        <div class="border rounded-lg overflow-hidden">
                            <button
                                class={classes!(theme.header, "w-full", "flex", "justify-between", "items-center", "font-bold", "py-2", "px-3", "text-left")}
                                onclick={on_toggle}
                                aria-expanded={(!is_collapsed).to_string()}
                            >
                                <span>{ line_name }</span>
                                <span aria-hidden="true">{ if is_collapsed { "▼" } else { "▲" } }</span>
                            </button>
                            if !is_collapsed {
                                { for platforms.iter().map(|(platform_name, platform_arrivals)| {
                                    let platform_id = format!("{line_name}-{platform_name}");
                                    let is_expanded = expanded_platforms.contains(&platform_id);
                                    let shown = if is_expanded {
                                        &platform_arrivals[..]
                                    } else {
                                        &platform_arrivals[..platform_arrivals.len().min(COLLAPSED_ARRIVALS)]
                                    };
                                    let toggle = toggle_platform.reform(move |_: MouseEvent| platform_id.clone());

                                    html! {
                                        <div class="border-t border-gray-200">
                                            <div class={classes!(theme.light, "font-medium", "py-2", "px-3", "flex", "items-center", "justify-between")}>
                                                <span class="text-gray-700">{ platform_name }</span>
                                                if platform_arrivals.len() > COLLAPSED_ARRIVALS {
                                                    <button
                                                        onclick={toggle}
                                                        aria-expanded={is_expanded.to_string()}
                                                        class="text-blue-600 hover:text-blue-800 text-sm px-2 py-1"
                                                    >
                                                        if is_expanded {
                                                            { "Show less" }
                                                        } else {
                                                            { format!("Show all ({})", platform_arrivals.len()) }
                                                        }
                                                    </button>
                                                }
                                            </div>
                                            { for shown.iter().enumerate().map(|(idx, arrival)| html! {
                                                <ArrivalRow
                                                    key={arrival.id.clone()}
                                                    arrival={arrival.clone()}
                                                    striped={idx % 2 == 1}
                                                    tracked={props.tracked_ids.contains(&arrival.id)}
                                                    on_track={props.on_track.clone()}
                                                />
                                            }) }
                                        </div>
                                    }
                                }) }
                            }
                        </div>
                    }
                }) }
            </div>
        },
    };

    html! {
        <div class="mt-4 p-3 sm:p-4 bg-blue-50 rounded">
            { header }
            if let Some(error) = &*error {
                <div class="mb-4 p-2 bg-red-50 text-red-700 rounded text-sm" role="alert">
                    { error }
                </div>
            }
            { body }
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct ArrivalRowProps {
    arrival: Arrival,
    striped: bool,
    tracked: bool,
    on_track: Callback<Arrival>,
}

#[function_component(ArrivalRow)]
fn arrival_row(props: &ArrivalRowProps) -> Html {
    let arrival = &props.arrival;
    let on_track = {
        let arrival = arrival.clone();
        props.on_track.reform(move |_: MouseEvent| arrival.clone())
    };

    let (button_class, title) = if props.tracked {
        (
            "bg-gray-300 text-gray-600 p-1 rounded cursor-not-allowed",
            "Already tracking this train",
        )
    } else if arrival.vehicle_id.is_some() {
        (
            mode_button_classes(&arrival.mode_name),
            "Track this train (Detailed tracking available)",
        )
    } else {
        (
            "bg-blue-100 text-blue-700 hover:bg-blue-200 p-1 rounded",
            "Track this train",
        )
    };

    html! {
        <div class={classes!(
            "flex", "justify-between", "items-center", "py-2", "px-2", "sm:px-3", "gap-2",
            if props.striped { "bg-gray-50" } else { "bg-white" },
        )}>
            <div class="flex-grow min-w-0">
                <div class="font-medium text-sm truncate">
                    { arrival.destination_name.as_deref().unwrap_or("Unknown") }
                </div>
                <div class="text-xs text-gray-600 truncate">{ &arrival.current_location }</div>
            </div>
            <div class="flex items-center shrink-0 gap-2">
                <div class="font-medium text-right">{ format_arrival_time(arrival.time_to_station) }</div>
                <button
                    onclick={on_track}
                    disabled={props.tracked}
                    class={button_class}
                    {title}
                    aria-label={title}
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                    </svg>
                </button>
            </div>
        </div>
    }
}
