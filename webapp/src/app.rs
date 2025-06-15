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
use web_sys::HtmlSelectElement;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
struct TrackedTrain {
    train: Arrival,
    station: StopPoint,
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
    let select_ref = use_node_ref();
    let tracked_trains = use_state(|| Vec::<TrackedTrain>::new());
    let expanded_platforms = use_state(|| HashMap::<String, bool>::new());
    let show_station_ids = use_state(|| false);

    // Initial loading of stations
    {
        let stations = stations.clone();
        let loading = loading.clone();
        let select_ref = select_ref.clone();

        use_effect_with((), move |_| {
            spawn_local(async move {
                let fetched = fetch_tube_stations().await;
                let grouped = group_stations_by_name(&fetched);
                stations.set(grouped);
                loading.set(false);

                // Reset select element after rendering
                gloo_timers::callback::Timeout::new(0, move || {
                    if let Some(select) = select_ref.cast::<HtmlSelectElement>() {
                        select.set_value("");
                    }
                })
                .forget();
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

    // Update station selection handler
    let on_select = {
        let stations = stations.clone();
        let selected_station = selected_station.clone();
        let fetching_data = fetching_data.clone();
        let arrivals = arrivals.clone();

        Callback::from(move |e: Event| {
            let input: HtmlSelectElement = e.target_unchecked_into();
            let id = input.value();

            if id.is_empty() {
                selected_station.set(None);
                return;
            }

            if let Some(group) = stations.iter().find(|s| s.id == id) {
                if let Some(station) = group.all_stations.first() {
                    selected_station.set(Some(station.clone()));

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
            }
        })
    };

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
                    <label class="block mb-2 font-medium">{ "Select a Station" }</label>
                    <select
                        ref={select_ref.clone()}
                        class="w-full p-2 border rounded"
                        onchange={on_select}
                        value=""
                    >
                        <option value="">{ "-- Choose a station --" }</option>
                        { for stations.iter().map(|s| html! {
                            <option value={s.id.clone()}>
                                { &s.common_name }
                            </option>
                        }) }
                    </select>
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
