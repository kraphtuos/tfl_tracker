use crate::station::{
    Arrival, GroupedStation, StopPoint, fetch_arrivals, fetch_tube_stations,
    group_arrivals_by_line_platform, group_stations_by_name,
};
use crate::train_tracker::TrainTracker;
use crate::utils::{
    format_arrival_time, get_line_background_colors, get_line_colors, get_mode_colors,
};
use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, KeyboardEvent};
use yew::prelude::*;

#[derive(Clone, PartialEq)]
struct TrackedTrain {
    train: Arrival,
    station: StopPoint,
}

// Add TypeaheadState to manage typeahead UI state
#[derive(Clone, PartialEq)]
struct TypeaheadState {
    query: String,
    focused_index: i32,
    show_suggestions: bool,
}

impl Default for TypeaheadState {
    fn default() -> Self {
        Self {
            query: String::new(),
            focused_index: -1,
            show_suggestions: false,
        }
    }
}

// Helper function to get station IDs for a station group
fn get_station_group_ids(station: &StopPoint, stations: &[GroupedStation]) -> Option<Vec<String>> {
    stations
        .iter()
        .find(|s| s.all_stations.iter().any(|st| st.id == station.id))
        .map(|group| group.all_stations.iter().map(|s| s.id.clone()).collect())
}

// Helper function to fetch arrivals for station IDs
async fn fetch_station_arrivals(station_ids: &[String]) -> Vec<Arrival> {
    fetch_arrivals(station_ids).await
}

#[function_component(App)]
pub fn app() -> Html {
    let stations = use_state(|| Vec::<GroupedStation>::new());
    let selected_station = use_state(|| None::<StopPoint>);
    let loading = use_state(|| true);
    let fetching_data = use_state(|| false);
    let arrivals = use_state(|| Vec::<Arrival>::new());
    let grouped_arrivals = use_state(|| Vec::new());
    let input_ref = use_node_ref();
    let tracked_trains = use_state(|| Vec::<TrackedTrain>::new());
    let expanded_platforms = use_state(|| HashMap::<String, bool>::new());
    let show_station_ids = use_state(|| false);
    let typeahead = use_state(TypeaheadState::default);

    // Initial loading of stations
    {
        let stations = stations.clone();
        let loading = loading.clone();

        use_effect_with((), move |_| {
            spawn_local(async move {
                let fetched = fetch_tube_stations().await;
                let grouped = group_stations_by_name(&fetched);
                stations.set(grouped);
                loading.set(false);
            });
            || ()
        });
    }

    // Update grouped arrivals whenever arrivals change
    {
        let arrivals = arrivals.clone();
        let grouped_arrivals = grouped_arrivals.clone();
        use_effect_with(arrivals.clone(), move |_| {
            let grouped = group_arrivals_by_line_platform(&arrivals);
            grouped_arrivals.set(grouped);
            || ()
        });
    }

    // Fetch arrivals whenever selected station changes
    {
        let selected_station = selected_station.clone();
        let fetching_data = fetching_data.clone();
        let arrivals = arrivals.clone();
        let stations = stations.clone();

        use_effect_with(selected_station.clone(), move |station| {
            if let Some(station) = &**station {
                if let Some(station_ids) = get_station_group_ids(station, &stations) {
                    fetching_data.set(true);
                    spawn_local({
                        let fetching_data = fetching_data.clone();
                        let arrivals = arrivals.clone();
                        async move {
                            let arrival_data = fetch_station_arrivals(&station_ids).await;
                            arrivals.set(arrival_data);
                            fetching_data.set(false);
                        }
                    });
                }
            }
            || ()
        });
    }

    // For refresh button
    let refresh_arrivals = {
        let selected_station = selected_station.clone();
        let stations = stations.clone();
        let fetching_data = fetching_data.clone();
        let arrivals = arrivals.clone();

        Callback::from(move |_e: MouseEvent| {
            if let Some(station) = &*selected_station {
                if let Some(station_ids) = get_station_group_ids(station, &stations) {
                    fetching_data.set(true);
                    spawn_local({
                        let fetching_data = fetching_data.clone();
                        let arrivals = arrivals.clone();
                        async move {
                            let arrival_data = fetch_station_arrivals(&station_ids).await;
                            arrivals.set(arrival_data);
                            fetching_data.set(false);
                        }
                    });
                }
            }
        })
    };

    // Track a train
    let track_train = {
        let tracked_trains = tracked_trains.clone();
        let selected_station = selected_station.clone();

        Callback::from(move |arrival: Arrival| {
            if let Some(station) = &*selected_station {
                tracked_trains.set({
                    let mut updated = (*tracked_trains).clone();
                    // Check if train is already being tracked
                    if !updated.iter().any(|t| t.train.id == arrival.id) {
                        updated.push(TrackedTrain {
                            train: arrival,
                            station: station.clone(),
                        });
                    }
                    updated
                });
            }
        })
    };

    // Remove a tracked train
    let remove_tracked_train = {
        let tracked_trains = tracked_trains.clone();

        Callback::from(move |index: usize| {
            tracked_trains.set({
                let mut updated = (*tracked_trains).clone();
                if index < updated.len() {
                    updated.remove(index);
                }
                updated
            });
        })
    };

    // Platform expansion toggle
    let toggle_platform = {
        let expanded_platforms = expanded_platforms.clone();

        Callback::from(move |platform_id: String| {
            expanded_platforms.set({
                let mut new_map = (*expanded_platforms).clone();
                let current = new_map.get(&platform_id).copied().unwrap_or(false);
                new_map.insert(platform_id, !current);
                new_map
            });
        })
    };

    // Station IDs toggle
    let toggle_station_ids = {
        let show_station_ids = show_station_ids.clone();
        Callback::from(move |_| {
            show_station_ids.set(!*show_station_ids);
        })
    };

    // Filter stations based on search query
    let filtered_stations = {
        let stations = stations.clone();
        let query = typeahead.query.clone();
        stations
            .iter()
            .filter(|station| {
                station
                    .common_name
                    .to_lowercase()
                    .contains(&query.to_lowercase())
            })
            .cloned()
            .collect::<Vec<_>>()
    };

    let on_input = {
        let typeahead = typeahead.clone();
        let selected_station = selected_station.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let new_value = input.value();
            let mut state = (*typeahead).clone();
            state.query = new_value.clone();
            state.show_suggestions = true;
            state.focused_index = -1;
            typeahead.set(state);

            // Clear selected station if query doesn't match station name
            if let Some(station) = &*selected_station {
                if station.common_name.to_lowercase() != new_value.to_lowercase() {
                    selected_station.set(None);
                }
            }
        })
    };

    let on_focus = {
        let typeahead = typeahead.clone();
        Callback::from(move |_| {
            let mut state = (*typeahead).clone();
            state.show_suggestions = true;
            typeahead.set(state);
        })
    };

    let on_blur = {
        let typeahead = typeahead.clone();
        let selected_station = selected_station.clone();
        Callback::from(move |_| {
            // Restore timeout to allow click events to complete
            let typeahead = typeahead.clone();
            let selected_station = selected_station.clone();
            gloo_timers::callback::Timeout::new(200, move || {
                let mut state = (*typeahead).clone();
                state.show_suggestions = false;
                if let Some(station) = &*selected_station {
                    state.query = station.common_name.clone();
                }
                typeahead.set(state);
            })
            .forget();
        })
    };

    let on_keydown = {
        let typeahead = typeahead.clone();
        let filtered_stations = filtered_stations.clone();
        let selected_station = selected_station.clone();
        Callback::from(move |e: KeyboardEvent| {
            let mut state = (*typeahead).clone();
            match e.key().as_str() {
                "ArrowDown" => {
                    e.prevent_default();
                    state.focused_index =
                        (state.focused_index + 1).min((filtered_stations.len() - 1) as i32);
                    typeahead.set(state);
                }
                "ArrowUp" => {
                    e.prevent_default();
                    state.focused_index = (state.focused_index - 1).max(-1);
                    typeahead.set(state);
                }
                "Enter" => {
                    if state.focused_index >= 0
                        && (state.focused_index as usize) < filtered_stations.len()
                    {
                        let station = &filtered_stations[state.focused_index as usize];
                        selected_station.set(Some(station.all_stations[0].clone()));
                        state.query = station.common_name.clone();
                        state.show_suggestions = false;
                        typeahead.set(state);
                    }
                }
                "Escape" => {
                    state.show_suggestions = false;
                    typeahead.set(state);
                }
                _ => {}
            }
        })
    };

    let select_station = {
        let selected_station = selected_station.clone();
        let typeahead = typeahead.clone();
        Callback::from(move |station: GroupedStation| {
            selected_station.set(Some(station.all_stations[0].clone()));
            let mut state = (*typeahead).clone();
            state.query = station.common_name.clone();
            state.show_suggestions = false;
            typeahead.set(state);
        })
    };

    // Clear input handler
    let clear_input = {
        let typeahead = typeahead.clone();
        let selected_station = selected_station.clone();
        Callback::from(move |_| {
            let mut state = (*typeahead).clone();
            state.query = String::new();
            state.show_suggestions = false;
            state.focused_index = -1;
            typeahead.set(state);
            selected_station.set(None);
        })
    };

    // Render station selection
    let station_selection = html! {
        <div class="relative w-full">
            <div class="relative">
                <input
                    ref={input_ref}
                    type="text"
                    placeholder="Search for a station..."
                    class="w-full p-3 text-base sm:text-lg border border-gray-200 rounded-lg bg-white text-gray-900 focus:ring-2 focus:ring-blue-500 focus:border-transparent pr-10"
                    value={typeahead.query.clone()}
                    oninput={on_input}
                    onfocus={on_focus}
                    onblur={on_blur}
                    onkeydown={on_keydown}
                />
                if !typeahead.query.is_empty() {
                    <button
                        onclick={clear_input}
                        onmousedown={Callback::from(|e: MouseEvent| e.prevent_default())}
                        class="absolute right-2 top-1/2 -translate-y-1/2 p-2 text-gray-500 hover:text-gray-700"
                        aria-label="Clear search"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                            <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
                        </svg>
                    </button>
                }
            </div>
            {
                if typeahead.show_suggestions && !filtered_stations.is_empty() {
                    html! {
                        <div
                            class="absolute z-50 w-full mt-1 bg-white border border-gray-200 rounded-lg shadow-lg max-h-[70vh] sm:max-h-60 overflow-y-auto"
                            onmousedown={Callback::from(|e: MouseEvent| e.prevent_default())} // Add this line
                        >
                            {
                                filtered_stations.iter().enumerate().map(|(index, station)| {
                                    let is_focused = index as i32 == typeahead.focused_index;
                                    let station_clone = station.clone();
                                    let onclick = select_station.reform(move |_| station_clone.clone());

                                    html! {
                                        <div
                                            key={station.id.clone()}
                                            class={classes!(                                                "p-2",
                                                "cursor-pointer",
                                                "text-gray-900",
                                                "hover:bg-gray-100",
                                                if is_focused { "bg-blue-100" } else { "" }
                                            )}
                                            onclick={onclick}
                                        >
                                            <div class="font-medium">{&station.common_name}</div>
                                            <div class="text-sm text-gray-500">
                                                {station.modes.join(" • ")}
                                            </div>
                                        </div>
                                    }
                                }).collect::<Html>()
                            }
                        </div>
                    }
                } else {
                    html! {}
                }
            }
        </div>
    };

    html! {
        <div class="container mx-auto p-2 sm:p-4">
            <h1 class="text-xl sm:text-2xl font-bold mb-4">{ "TfL Tracker" }</h1>

            if !tracked_trains.is_empty() {
                <div class="mb-6">
                    <h2 class="text-lg font-semibold mb-4">{ "Tracked Trains" }</h2>
                    <div class="space-y-3 sm:space-y-4">
                        { for tracked_trains.iter().enumerate().map(|(idx, tracked)| {
                            let on_close = {
                                let idx = idx;
                                let remove_tracked_train = remove_tracked_train.clone();
                                Callback::from(move |_| {
                                    remove_tracked_train.emit(idx);
                                })
                            };

                            html! {
                                <TrainTracker
                                    key={tracked.train.id.clone()}
                                    train={tracked.train.clone()}
                                    station={tracked.station.clone()}
                                    on_close={on_close}
                                />
                            }
                        }) }
                    </div>
                </div>
            }

            if *loading {
                <div class="flex items-center justify-center p-4">
                    <LoadingSpinner />
                    <p class="ml-2">{ "Loading stations..." }</p>
                </div>
            } else {
                <div class="mb-4 sm:mb-6">
                    <label class="block mb-2 font-medium">{ "Choose a Station" }</label>
                    {station_selection}
                </div>

                if let Some(station) = &*selected_station {
                    <div class="mt-4 p-3 sm:p-4 bg-blue-50 rounded">
                        <div class="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-3 mb-4">
                            <div>
                                <h2 class="text-lg sm:text-xl font-semibold">{ &station.common_name }</h2>
                                <div class="text-gray-600 text-sm">
                                    {
                                        if let Some(group) = stations.iter().find(|s| s.all_stations.iter().any(|st| st.id == station.id)) {
                                            let station_ids = group.all_stations.iter()
                                                .map(|s| format!("{} ({})", s.id, s.mode_name))
                                                .collect::<Vec<_>>();
                                            html! {
                                                <>
                                                    <button
                                                        onclick={toggle_station_ids.clone()}
                                                        class="text-blue-600 hover:text-blue-800 text-sm py-1"
                                                    >
                                                        { if *show_station_ids {
                                                            "Hide Station IDs".to_string()
                                                        } else {
                                                            format!("Show Station IDs ({})", station_ids.len())
                                                        }}
                                                    </button>
                                                    if *show_station_ids {
                                                        <div class="mt-2 ml-2 text-sm space-y-1">
                                                            { for station_ids.iter().map(|id| html! {
                                                                <div>{ id }</div>
                                                            }) }
                                                        </div>
                                                    }
                                                </>
                                            }
                                        } else {
                                            html! {
                                                <p>{ format!("Station ID: {}", station.id) }</p>
                                            }
                                        }
                                    }
                                </div>
                            </div>
                            <button
                                onclick={refresh_arrivals}
                                class="w-full sm:w-auto bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
                                disabled={*fetching_data}
                            >
                                if *fetching_data {
                                    <span class="flex items-center justify-center">
                                        <LoadingSpinner />
                                        <span class="ml-2">{ "Updating..." }</span>
                                    </span>
                                } else {
                                    { "Refresh Arrivals" }
                                }
                            </button>
                        </div>

                        if *fetching_data {
                            <div class="flex items-center justify-center p-4 sm:p-6">
                                <LoadingSpinner />
                                <p class="ml-2">{ "Fetching arrival information..." }</p>
                            </div>
                        } else if !arrivals.is_empty() {
                            <div class="space-y-4 sm:space-y-6">
                                { for grouped_arrivals.iter().map(|(line_name, platforms)| html! {
                                    <div class="border rounded-lg overflow-hidden">
                                        <div class={get_line_colors(&line_name)}>
                                            { line_name }
                                        </div>
                                        <div>
                                            { for platforms.iter().map(|(platform_name, platform_arrivals)| {
                                                let platform_id = format!("{}-{}", line_name, platform_name);
                                                let is_expanded = expanded_platforms.get(&platform_id).copied().unwrap_or(false);
                                                let display_arrivals = if is_expanded {
                                                    platform_arrivals
                                                } else {
                                                    &platform_arrivals[0..std::cmp::min(3, platform_arrivals.len())]
                                                };
                                                let toggle = {
                                                    let platform_id = platform_id.clone();
                                                    let toggle_platform = toggle_platform.clone();
                                                    Callback::from(move |_| toggle_platform.emit(platform_id.clone()))
                                                };

                                                html! {                                                    <div class="border-t border-gray-200">                                                        <div class={format!("{} flex items-center justify-between", get_line_background_colors(&line_name))}>
                                                            <span class="text-gray-700">{ platform_name }</span>
                                                            if platform_arrivals.len() > 3 {
                                                                <button
                                                                    onclick={toggle}
                                                                    class="text-blue-600 hover:text-blue-800 text-sm px-2 py-1"
                                                                >
                                                                    if is_expanded {
                                                                        { "Show less" }
                                                                    } else {
                                                                        { format!("Show all ({})", platform_arrivals.len()) }
                                                                    }
                                                                </button>
                                                            }
                                                        </div>                                                <div>
                                                            { for display_arrivals.iter().enumerate().map(|(idx, arrival)| {
                                                                let arrival_clone = arrival.clone();
                                                                let on_track = {
                                                                    let track_train = track_train.clone();
                                                                    let arrival = arrival.clone();

                                                                    Callback::from(move |_| {
                                                                        track_train.emit(arrival.clone());
                                                                    })
                                                                };

                                                                html! {                                                    <div class={classes!(
                                                                        "flex",
                                                                        "flex-col",
                                                                        "sm:flex-row",
                                                                        "sm:justify-between",
                                                                        "items-start",
                                                                        "sm:items-center",
                                                                        "py-2",
                                                                        "px-2",
                                                                        "sm:px-3",
                                                                        "gap-2",
                                                                        "sm:gap-0",
                                                                        if idx % 2 == 0 { "bg-white" } else { "bg-gray-50" }
                                                                    )}>
                                                                        <div class="flex-grow">
                                                                            <div class="font-medium text-sm sm:text-base">{ arrival_clone.towards.unwrap_or_else(|| "Unknown".to_string()) }</div>
                                                                            <div class="text-xs sm:text-sm text-gray-600">{ &arrival_clone.current_location }</div>
                                                                        </div>                                                                        <div class="flex items-center justify-between sm:justify-end w-full sm:w-auto gap-2">
                                                                            <div class="font-medium text-right">
                                                                                { format_arrival_time(arrival_clone.time_to_station) }
                                                                            </div>
                                                                            <button
                                                                                onclick={on_track}
                                                                                disabled={tracked_trains.iter().any(|t| t.train.id == arrival_clone.id)}
                                                                                class={
                                                                                    if tracked_trains.iter().any(|t| t.train.id == arrival_clone.id) {
                                                                                        "bg-gray-300 text-gray-600 p-1 rounded cursor-not-allowed".to_string()
                                                                                    } else if arrival_clone.vehicle_id.is_some() {
                                                                                        get_mode_colors(&arrival_clone.mode_name)
                                                                                    } else {
                                                                                        "bg-blue-100 text-blue-700 hover:bg-blue-200 p-1 rounded".to_string()
                                                                                    }
                                                                                }
                                                                                title={
                                                                                    if arrival_clone.vehicle_id.is_some() {
                                                                                        "Track this train (Detailed tracking available)"
                                                                                    } else {
                                                                                        "Track this train"
                                                                                    }
                                                                                }
                                                                            >
                                                                                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                                                                                </svg>
                                                                            </button>
                                                                        </div>
                                                                    </div>
                                                                }
                                                            }) }
                                                        </div>
                                                    </div>
                                                }
                                            }) }
                                        </div>
                                    </div>
                                }) }
                            </div>
                        } else {
                            <div class="text-center p-4 sm:p-6 text-gray-500">
                                { "No arrival information available. Click 'Refresh Arrivals' to load data." }
                            </div>
                        }
                    </div>
                }
            }
        </div>
    }
}

#[function_component(LoadingSpinner)]
fn loading_spinner() -> Html {
    html! {
        <div class="inline-block animate-spin rounded-full h-5 w-5 border-t-2 border-b-2 border-blue-500"></div>
    }
}
