use crate::station::{Arrival, StopPoint, VehicleArrival, fetch_arrivals, fetch_vehicle_arrivals};
use gloo_timers::callback::Interval;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct TrainTrackerProps {
    pub train: Arrival,
    pub station: StopPoint,
    pub on_close: Callback<()>,
    #[prop_or_default]
    pub key: String, // Added key prop for uniqueness
}

#[derive(Clone, Debug, PartialEq)]
struct NextStop {
    station_name: String,
    time_to_station: i32,
}

#[function_component(TrainTracker)]
pub fn train_tracker(props: &TrainTrackerProps) -> Html {
    // Use local state for each train tracker
    let train = props.train.clone();
    let station = props.station.clone();

    // Define component-specific state
    let current_location = use_state(|| props.train.current_location.clone());
    let time_to_station = use_state(|| props.train.time_to_station);
    let status = use_state(|| "Approaching".to_string());
    let loading = use_state(|| false);
    let next_stops = use_state(|| Vec::<NextStop>::new());
    let vehicle_id = use_state(|| props.train.vehicle_id.clone().unwrap_or_default());
    let line_id = use_state(|| props.train.line_id.clone().unwrap_or_default());
    let error_message = use_state(|| String::new());

    // Use a Rc to store the interval so it can be dropped properly on unmount
    let track_interval = use_state(|| None::<Rc<Interval>>);

    // Function to update train status using vehicle API
    let update_train_status = {
        let loading = loading.clone();
        let current_location = current_location.clone();
        let time_to_station = time_to_station.clone();
        let status = status.clone();
        let vehicle_id = vehicle_id.clone();
        let line_id = line_id.clone();
        let next_stops = next_stops.clone();
        let error_message = error_message.clone();
        let station_id = station.id.clone();
        let train_id = train.id.clone();

        Callback::from(move |_| {
            loading.set(true);
            error_message.set(String::new());

            // If we don't have a vehicle ID yet, try to get it from the station arrivals
            if vehicle_id.is_empty() {
                let train_id = train_id.clone();
                let station_id = station_id.clone();
                let loading = loading.clone();
                let vehicle_id = vehicle_id.clone();
                let line_id = line_id.clone();
                let error_message = error_message.clone();

                spawn_local(async move {
                    let arrivals = fetch_arrivals(&[station_id]).await;

                    // Find our train in the arrivals
                    if let Some(arrival) = arrivals.iter().find(|a| a.id == train_id) {
                        if let Some(v_id) = &arrival.vehicle_id {
                            vehicle_id.set(v_id.clone());
                            if let Some(l_id) = &arrival.line_id {
                                line_id.set(l_id.clone());
                            }
                        } else {
                            error_message.set("Train no longer found at station".to_string());
                        }
                    } else {
                        error_message.set("Train no longer found at station".to_string());
                    }
                    loading.set(false);
                });
                return;
            }

            // If we have a vehicle ID, use the vehicle arrivals API
            let vehicle_id_value = vehicle_id.to_string();
            let line_id_value = line_id.to_string();
            let loading = loading.clone();
            let current_location = current_location.clone();
            let time_to_station = time_to_station.clone();
            let status = status.clone();
            let next_stops = next_stops.clone();
            let error_message = error_message.clone();

            spawn_local(async move {
                if vehicle_id_value.is_empty() {
                    error_message.set("No vehicle ID available".to_string());
                    loading.set(false);
                    return;
                }

                let vehicle_arrivals = fetch_vehicle_arrivals(&vehicle_id_value).await;

                // Filter by line ID
                let filtered_arrivals: Vec<VehicleArrival> = vehicle_arrivals
                    .into_iter()
                    .filter(|a| a.line_id == line_id_value)
                    .collect();

                if filtered_arrivals.is_empty() {
                    error_message.set("No predictions found for this train".to_string());
                    loading.set(false);
                    return;
                }

                // Sort by time to station
                let mut sorted_arrivals = filtered_arrivals.clone();
                sorted_arrivals.sort_by(|a, b| a.time_to_station.cmp(&b.time_to_station));

                // Get current location from the first arrival
                if !sorted_arrivals.is_empty() {
                    let first = &sorted_arrivals[0];
                    current_location.set(first.current_location.clone());
                    time_to_station.set(first.time_to_station);

                    // Update status based on time to station
                    if first.time_to_station < 30 {
                        status.set("Arriving".to_string());
                    } else if first.time_to_station < 60 {
                        status.set("Approaching".to_string());
                    } else {
                        status.set("En Route".to_string());
                    }

                    // Take the next 5 stops (or fewer if there aren't 5)
                    let stops_count = std::cmp::min(5, sorted_arrivals.len());
                    let next_5_stops: Vec<NextStop> = sorted_arrivals[0..stops_count]
                        .iter()
                        .map(|a| NextStop {
                            station_name: a.station_name.clone(),
                            time_to_station: a.time_to_station,
                        })
                        .collect();

                    next_stops.set(next_5_stops);
                }

                loading.set(false);
            });
        })
    };

    // Start tracking interval when component mounts
    {
        let update_train_status = update_train_status.clone();
        let track_interval = track_interval.clone();

        use_effect_with((), move |_| {
            // Call once immediately for initial data
            update_train_status.emit(());

            // Set up interval to update every 15 seconds
            let interval = Interval::new(15_000, move || {
                update_train_status.emit(());
            });

            // Store the interval in an Rc so we can safely drop it on unmount
            track_interval.set(Some(Rc::new(interval)));

            // Cleanup function to cancel the interval when the component unmounts
            || {
                // The interval will be dropped when track_interval is dropped
            }
        });
    }

    let format_time = |seconds: i32| -> String {
        if seconds < 60 {
            format!("Due")
        } else {
            let minutes = seconds / 60;
            format!("{} min", minutes)
        }
    };

    let on_close = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| {
            on_close.emit(());
        })
    };

    // Get status color based on status text
    let status_color = match status.as_str() {
        "Arrived" => "bg-green-100 text-green-800",
        "Arriving" => "bg-yellow-100 text-yellow-800",
        "Approaching" => "bg-blue-100 text-blue-800",
        "Not Found" => "bg-red-100 text-red-800",
        _ => "bg-gray-100 text-gray-800",
    };

    let update_train_status_click = {
        let update_train_status = update_train_status.clone();
        Callback::from(move |_e: web_sys::MouseEvent| update_train_status.emit(()))
    };

    html! {
        <div class="border rounded-lg shadow-md p-4 mt-4 bg-white">
            <div class="flex justify-between items-center mb-3">
                <h3 class="text-lg font-bold">{ format!("Train Tracker: {} Line", train.line_name) }</h3>
                <button
                    onclick={on_close}
                    class="bg-gray-200 hover:bg-gray-300 rounded-full p-1"
                    title="Close tracker"
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            </div>

            <div class="grid grid-cols-2 gap-4 mb-4">
                <div>
                    <p class="text-sm text-gray-500">{"Destination"}</p>
                    <p class="font-medium">{ train.towards.as_deref().unwrap_or("Unknown") }</p>
                </div>
                <div>
                    <p class="text-sm text-gray-500">{"Platform"}</p>
                    <p class="font-medium">{ &train.platform_name }</p>
                </div>
                <div>
                    <p class="text-sm text-gray-500">{"Current Location"}</p>
                    <p class="font-medium">{ &*current_location }</p>
                </div>
                <div>
                    <p class="text-sm text-gray-500">{"Next Arrival"}</p>
                    <p class="font-medium">{ format_time(*time_to_station) }</p>
                </div>
            </div>

            // Display next stops
            if !next_stops.is_empty() {
                <div class="mb-4">
                    <h4 class="font-medium mb-2">{"Next Stops"}</h4>
                    <div class="bg-gray-50 rounded p-2">
                        <table class="w-full text-sm">
                            <thead>
                                <tr class="border-b">
                                    <th class="text-left py-1">{"Station"}</th>
                                    <th class="text-right py-1">{"Arrival"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                { for next_stops.iter().map(|stop| html! {
                                    <tr class="border-b border-gray-100">
                                        <td class="py-1">{ &stop.station_name }</td>
                                        <td class="text-right py-1">{ format_time(stop.time_to_station) }</td>
                                    </tr>
                                }) }
                            </tbody>
                        </table>
                    </div>
                </div>
            }

            // Show error message if any
            if !(*error_message).is_empty() {
                <div class="mb-4 p-2 bg-red-50 text-red-700 rounded text-sm">
                    { &*error_message }
                </div>
            }

            <div class="flex justify-between items-center">
                <div class={format!("px-3 py-1 rounded-full text-sm font-medium {}", status_color)}>
                    { &*status }
                </div>

                <button
                    onclick={update_train_status_click}
                    disabled={*loading}
                    class="text-blue-600 hover:text-blue-800 text-sm flex items-center"
                >
                    if *loading {
                        <div class="mr-1 h-4 w-4 border-t-2 border-b-2 border-blue-600 rounded-full animate-spin"></div>
                        { "Updating..." }
                    } else {
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-1" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                        { "Refresh" }
                    }
                </button>
            </div>
        </div>
    }
}
