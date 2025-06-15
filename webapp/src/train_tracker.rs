use crate::station::{Arrival, StopPoint, VehicleArrival, fetch_arrivals, fetch_vehicle_arrivals};
use crate::utils::{format_arrival_time, get_line_theme_colors, get_status_color};
use gloo_timers::callback::Interval;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct TrainTrackerProps {
    pub train: Arrival,
    pub station: StopPoint,
    pub on_close: Callback<()>,
    pub is_collapsed: bool,
    pub on_toggle: Callback<()>,
}

#[derive(Clone, Debug, PartialEq)]
struct NextStop {
    station_name: String,
    time_to_station: i32,
}

// New type to encapsulate train status
#[derive(Clone)]
struct TrainStatus {
    current_location: String,
    time_to_station: i32,
    status: String,
    vehicle_id: String,
    line_id: String,
    next_stops: Vec<NextStop>,
}

impl Default for TrainStatus {
    fn default() -> Self {
        Self {
            current_location: String::new(),
            time_to_station: 0,
            status: "Approaching".to_string(),
            vehicle_id: String::new(),
            line_id: String::new(),
            next_stops: Vec::new(),
        }
    }
}

#[function_component(TrainTracker)]
pub fn train_tracker(props: &TrainTrackerProps) -> Html {
    // Add new state for Train ID visibility
    let show_train_id = use_state(|| false);
    let train = props.train.clone();
    let station = props.station.clone();
    let train_status = use_state(|| TrainStatus {
        current_location: props.train.current_location.clone(),
        time_to_station: props.train.time_to_station,
        status: "Approaching".to_string(),
        vehicle_id: props.train.vehicle_id.clone().unwrap_or_default(),
        line_id: props.train.line_id.clone().unwrap_or_default(),
        next_stops: Vec::new(),
    });

    let loading = use_state(|| false);
    let error_message = use_state(|| String::new());
    let track_interval = use_state(|| None::<Rc<Interval>>);

    // Function to update train status using vehicle API
    let update_train_status = {
        let loading = loading.clone();
        let error_message = error_message.clone();
        let train_status = train_status.clone();
        let train_id = train.id.clone();
        let station_id = station.id.clone();

        Callback::from(move |_| {
            let train_status = train_status.clone();
            let loading = loading.clone();
            let error_message = error_message.clone();
            let train_id = train_id.clone();
            let station_id = station_id.clone();

            loading.set(true);
            error_message.set(String::new());
            let mut current_status = (*train_status).clone();

            // If we don't have a vehicle ID yet, try to get it from the station arrivals
            if current_status.vehicle_id.is_empty() {
                let train_id = train_id.clone();
                let station_id = station_id.clone();
                spawn_local(async move {
                    let arrivals = fetch_arrivals(&[station_id]).await;
                    if let Some(arrival) = arrivals.iter().find(|a| a.id == train_id) {
                        if let Some(v_id) = &arrival.vehicle_id {
                            current_status.vehicle_id = v_id.clone();
                            if let Some(l_id) = &arrival.line_id {
                                current_status.line_id = l_id.clone();
                            }
                            train_status.set(current_status);
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
            let vehicle_id = current_status.vehicle_id.clone();
            let line_id = current_status.line_id.clone();
            spawn_local(async move {
                let vehicle_arrivals = fetch_vehicle_arrivals(&vehicle_id).await;

                let filtered_arrivals: Vec<VehicleArrival> = vehicle_arrivals
                    .into_iter()
                    .filter(|a| a.line_id == line_id)
                    .collect();

                if filtered_arrivals.is_empty() {
                    error_message.set("No predictions found for this train".to_string());
                    loading.set(false);
                    return;
                }

                let mut sorted_arrivals = filtered_arrivals.clone();
                sorted_arrivals.sort_by(|a, b| a.time_to_station.cmp(&b.time_to_station));

                current_status.current_location = sorted_arrivals[0].current_location.clone();
                current_status.time_to_station = sorted_arrivals[0].time_to_station;

                // Update status based on time to station
                current_status.status = if sorted_arrivals[0].time_to_station <= 30 {
                    "Arrived".to_string()
                } else if sorted_arrivals[0].time_to_station <= 60 {
                    "Arriving".to_string()
                } else {
                    "Approaching".to_string()
                };

                // Get next 5 stops
                let next_5_stops: Vec<NextStop> = sorted_arrivals
                    .iter()
                    .take(5)
                    .map(|arrival| NextStop {
                        station_name: arrival.station_name.clone(),
                        time_to_station: arrival.time_to_station,
                    })
                    .collect();

                current_status.next_stops = next_5_stops;
                train_status.set(current_status);
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

    let on_close = {
        let on_close = props.on_close.clone();
        Callback::from(move |_| {
            on_close.emit(());
        })
    };

    let update_train_status_click = {
        let update_train_status = update_train_status.clone();
        Callback::from(move |_e: MouseEvent| update_train_status.emit(()))
    };

    let (bg_color, border_color) = get_line_theme_colors(&train.line_name);
    let status_color = get_status_color(&train_status.status);

    // Create a properly cloned callback for the toggle
    let toggle = {
        let on_toggle = props.on_toggle.clone();
        move |_| on_toggle.emit(())
    };

    // Add toggle callback for Train ID
    let toggle_train_id = {
        let show_train_id = show_train_id.clone();
        Callback::from(move |_| {
            show_train_id.set(!*show_train_id);
        })
    };

    html! {
        <div class="rounded-lg shadow-md overflow-hidden">
            <div
                class={classes!("flex", "items-center", "justify-between", "p-4", "cursor-pointer", border_color, bg_color)}
                onclick={toggle}
            >
                <div class="flex items-center gap-2">
                    <span class="font-medium">
                        {&props.train.destination_name.clone().unwrap_or_else(|| "Unknown".to_string())}
                    </span>
                    <span class="text-sm text-gray-500">
                        {&props.train.current_location}
                    </span>
                </div>
                <div class="flex items-center gap-2">
                    <span>
                        if props.is_collapsed {
                            { "▼" }
                        } else {
                            { "▲" }
                        }
                    </span>
                    <button
                        onclick={on_close}
                        class="ml-2 p-1 text-gray-400 hover:text-gray-600"
                        title="Stop tracking"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                            <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
                        </svg>
                    </button>
                </div>
            </div>
            if !props.is_collapsed {
                <div class="p-4">
                    <div class="grid grid-cols-2 gap-4 mb-4">
                        <div>
                            <p class="text-sm text-gray-500">{"Destination"}</p>
                            <p class="font-medium">{ train.destination_name.as_deref().unwrap_or("Unknown") }</p>
                        </div>
                        <div>
                            <p class="text-sm text-gray-500">{"Platform"}</p>
                            <p class="font-medium">{ &train.platform_name }</p>
                        </div>
                        <div>
                            <p class="text-sm text-gray-500">{"Current Location"}</p>
                            <p class="font-medium">{ &train_status.current_location }</p>
                        </div>
                        <div>
                            <p class="text-sm text-gray-500">{"Next Arrival"}</p>
                            <p class="font-medium">{ format_arrival_time(train_status.time_to_station) }</p>
                        </div>
                        <div class="col-span-2">
                            <button
                                onclick={toggle_train_id}
                                class="text-blue-600 hover:text-blue-800 text-sm py-1"
                            >
                                { if *show_train_id {
                                    "Hide Train ID"
                                } else {
                                    "Show Train ID"
                                }}
                            </button>
                            if *show_train_id {
                                <p class="mt-1 text-sm text-gray-600">
                                    {
                                        if !train_status.vehicle_id.is_empty() {
                                            train_status.vehicle_id.to_string()
                                        } else {
                                            train.id.clone()
                                        }
                                    }
                                </p>
                            }
                        </div>
                    </div>

                    // Display next stops
                    if !train_status.next_stops.is_empty() {
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
                                        { for train_status.next_stops.iter().map(|stop| html! {
                                            <tr class="border-b border-gray-100">
                                                <td class="py-1">{ &stop.station_name }</td>
                                                <td class="text-right py-1">{ format_arrival_time(stop.time_to_station) }</td>
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
                            { &train_status.status }
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
        </div>
    }
}
