use crate::station::{
    Arrival, StopPoint, fetch_arrivals, fetch_tube_stations, group_arrivals_by_line_platform,
};
use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

#[function_component(App)]
pub fn app() -> Html {
    let stations = use_state(Vec::new);
    let selected_station = use_state(|| None::<StopPoint>);
    let loading = use_state(|| true);
    let fetching_data = use_state(|| false);
    let arrivals = use_state(|| Vec::new());
    let grouped_arrivals = use_state(|| HashMap::new());
    let select_ref = use_node_ref();

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

                // Reset select to default option after stations are loaded
                if let Some(select) = select_ref.cast::<HtmlSelectElement>() {
                    select.set_value("");
                }
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

        Callback::from(move |e: Event| {
            let input: HtmlSelectElement = e.target_unchecked_into();
            let id = input.value();
            if id.is_empty() {
                selected_station.set(None);
                return;
            }

            if let Some(station) = stations.iter().find(|s| s.id == id) {
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
                                                        { for platform_arrivals.iter().map(|arrival| html! {
                                                            <div class="flex justify-between items-center py-2 px-3">
                                                                <div>
                                                                    <div class="font-medium">{ &arrival.towards }</div>
                                                                    <div class="text-sm text-gray-600">{ &arrival.current_location }</div>
                                                                </div>
                                                                <div class="font-medium text-right">
                                                                    { format_time(arrival.time_to_station) }
                                                                </div>
                                                            </div>
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
