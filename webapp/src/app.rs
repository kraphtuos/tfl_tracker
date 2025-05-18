use crate::station::{
    Arrival, StopPoint, fetch_arrivals, fetch_tube_stations, group_arrivals_by_line_platform,
};
use crate::train_tracker::TrainTracker;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

#[function_component(App)]
pub fn app() -> Html {
    let stations = use_state(|| Vec::<StopPoint>::new());
    let selected_station = use_state(|| None::<StopPoint>);
    let loading = use_state(|| true);
    let fetching_data = use_state(|| false);
    let arrivals = use_state(|| Vec::<Arrival>::new());
    let grouped_arrivals = use_state(|| Vec::new());
    let select_ref = use_node_ref();
    let tracked_trains = use_state(|| Vec::<Arrival>::new());

    // Initial loading of stations
    {
        let stations = stations.clone();
        let loading = loading.clone();
        let select_ref = select_ref.clone();

        use_effect_with((), move |_| {
            spawn_local(async move {
                let fetched = fetch_tube_stations().await;
                stations.set(fetched);
                loading.set(false);

                // Use a timeout to ensure the select element is reset to default after rendering
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

        use_effect_with(selected_station.clone(), move |station| {
            if let Some(station) = &**station {
                let station_id = station.id.clone();
                fetching_data.set(true);

                spawn_local(async move {
                    let arrival_data = fetch_arrivals(&station_id).await;
                    arrivals.set(arrival_data);
                    fetching_data.set(false);
                });
            }
            || ()
        });
    }

    let fetch_station_arrivals = {
        let selected_station = selected_station.clone();
        let fetching_data = fetching_data.clone();
        let arrivals = arrivals.clone();

        Callback::from(move |_| {
            if let Some(station) = &*selected_station {
                let station_id = station.id.clone();
                let fetching_data = fetching_data.clone();
                let arrivals = arrivals.clone();

                fetching_data.set(true);
                spawn_local(async move {
                    let arrival_data = fetch_arrivals(&station_id).await;
                    arrivals.set(arrival_data);
                    fetching_data.set(false);
                });
            }
        })
    };

    let on_select = {
        let stations = stations.clone();
        let selected_station = selected_station.clone();
        let tracked_trains = tracked_trains.clone();

        Callback::from(move |e: Event| {
            let input: HtmlSelectElement = e.target_unchecked_into();
            let id = input.value();
            if id.is_empty() {
                selected_station.set(None);
                return;
            }

            if let Some(station) = stations.iter().find(|s| s.id == id) {
                // Clear tracked trains when changing stations
                tracked_trains.set(Vec::new());
                // Just set the selected station - effect hook will trigger the fetch
                selected_station.set(Some(station.clone()));
            }
        })
    };

    // For refresh button
    let refresh_arrivals = {
        let fetch_station_arrivals = fetch_station_arrivals.clone();
        Callback::from(move |_e: MouseEvent| fetch_station_arrivals.emit(()))
    };

    // Track a train
    let track_train = {
        let tracked_trains = tracked_trains.clone();

        Callback::from(move |arrival: Arrival| {
            tracked_trains.set({
                let mut updated = (*tracked_trains).clone();
                // Check if train is already being tracked
                if !updated.iter().any(|t| t.id == arrival.id) {
                    updated.push(arrival);
                }
                updated
            });
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

    let format_time = |seconds: i32| -> String {
        if seconds < 60 {
            format!("Due")
        } else {
            let minutes = seconds / 60;
            format!("{} min", minutes)
        }
    };

    html! {
        <div class="container mx-auto p-4">
            <h1 class="text-2xl font-bold mb-4">{ "London Underground Tracker" }</h1>

            if *loading {
                <div class="flex items-center justify-center p-4">
                    <LoadingSpinner />
                    <p class="ml-2">{ "Loading stations..." }</p>
                </div>
            } else {
                <div class="mb-6">
                    <label class="block mb-2 font-medium">{ "Select a Tube Station" }</label>
                    <select
                        ref={select_ref.clone()}
                        class="w-full p-2 border rounded"
                        onchange={on_select}
                        value=""
                    >
                        <option value="">{ "-- Choose a station --" }</option>
                        { for stations.iter().map(|s| html! {
                            <option value={s.id.clone()}>{ &s.common_name }</option>
                        }) }
                    </select>
                </div>

                if let Some(station) = &*selected_station {
                    <div class="mt-4 p-4 bg-blue-50 rounded">
                        <div class="flex justify-between items-center mb-4">
                            <div>
                                <h2 class="text-xl font-semibold">{ &station.common_name }</h2>
                                <p class="text-gray-600 text-sm">{ format!("Station ID: {}", station.id) }</p>
                            </div>
                            <button
                                onclick={refresh_arrivals}
                                class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
                                disabled={*fetching_data}
                            >
                                if *fetching_data {
                                    <span class="flex items-center">
                                        <LoadingSpinner />
                                        <span class="ml-2">{ "Updating..." }</span>
                                    </span>
                                } else {
                                    { "Refresh Arrivals" }
                                }
                            </button>
                        </div>

                        // Track Train section
                        if !tracked_trains.is_empty() {
                            <div class="mb-6 space-y-4">
                                <h3 class="text-lg font-semibold">{ "Tracked Trains" }</h3>
                                { for tracked_trains.iter().enumerate().map(|(idx, train)| {
                                    let on_close = {
                                        let idx = idx;
                                        let remove_tracked_train = remove_tracked_train.clone();
                                        Callback::from(move |_| {
                                            remove_tracked_train.emit(idx);
                                        })
                                    };

                                    html! {
                                        <TrainTracker
                                            key={train.id.clone()}
                                            train={train.clone()}
                                            station={station.clone()}
                                            on_close={on_close}
                                        />
                                    }
                                }) }
                            </div>
                        }

                        if *fetching_data {
                            <div class="flex items-center justify-center p-6">
                                <LoadingSpinner />
                                <p class="ml-2">{ "Fetching arrival information..." }</p>
                            </div>
                        } else if !arrivals.is_empty() {
                            <div class="space-y-6">
                                { for grouped_arrivals.iter().map(|(line_name, platforms)| html! {
                                    <div class="border rounded-lg overflow-hidden">
                                        <div class="bg-gray-100 font-bold py-2 px-3">
                                            { line_name }
                                        </div>
                                        <div>
                                            { for platforms.iter().map(|(platform_name, platform_arrivals)| html! {
                                                <div class="border-t border-gray-200">
                                                    <div class="bg-gray-50 py-1 px-3 font-medium">
                                                        { format!("Platform {}", platform_name) }
                                                    </div>
                                                    <div class="divide-y divide-gray-100">
                                                        { for platform_arrivals.iter().map(|arrival| {
                                                            let arrival_clone = arrival.clone();
                                                            let on_track = {
                                                                let track_train = track_train.clone();
                                                                let arrival = arrival.clone();

                                                                Callback::from(move |_| {
                                                                    // Clone and emit the arrival for tracking
                                                                    track_train.emit(arrival.clone());
                                                                })
                                                            };

                                                            html! {
                                                                <div class="flex justify-between items-center py-2 px-3">
                                                                    <div class="flex-grow">
                                                                        <div class="font-medium">{ &arrival_clone.towards }</div>
                                                                        <div class="text-sm text-gray-600">{ &arrival_clone.current_location }</div>
                                                                    </div>
                                                                    <div class="flex items-center">
                                                                        <div class="font-medium text-right mr-3">
                                                                            { format_time(arrival_clone.time_to_station) }
                                                                        </div>
                                                                        <button
                                                                            onclick={on_track}
                                                                            disabled={tracked_trains.iter().any(|t| t.id == arrival_clone.id)}
                                                                            class={
                                                                                if tracked_trains.iter().any(|t| t.id == arrival_clone.id) {
                                                                                    "bg-gray-300 text-gray-600 p-1 rounded cursor-not-allowed"
                                                                                } else if arrival_clone.vehicle_id.is_some() {
                                                                                    "bg-green-100 text-green-700 hover:bg-green-200 p-1 rounded"
                                                                                } else {
                                                                                    "bg-blue-100 text-blue-700 hover:bg-blue-200 p-1 rounded"
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
                                            }) }
                                        </div>
                                    </div>
                                }) }
                            </div>
                        } else {
                            <div class="text-center p-6 text-gray-500">
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
