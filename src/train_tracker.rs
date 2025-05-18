use crate::station::{Arrival, StopPoint, fetch_arrivals};
use gloo_timers::callback::Interval;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct TrainTrackerProps {
    pub train: Arrival,
    pub station: StopPoint,
    pub on_close: Callback<()>,
}

#[function_component(TrainTracker)]
pub fn train_tracker(props: &TrainTrackerProps) -> Html {
    let train = use_state(|| props.train.clone());
    let station = props.station.clone();
    let current_location = use_state(|| props.train.current_location.clone());
    let time_to_station = use_state(|| props.train.time_to_station);
    let status = use_state(|| "Approaching".to_string());
    let loading = use_state(|| false);
    let track_interval = use_state(|| None::<Interval>);

    // Function to update train status
    let update_train_status = {
        let train = train.clone();
        let station_id = station.id.clone();
        let loading = loading.clone();
        let current_location = current_location.clone();
        let time_to_station = time_to_station.clone();
        let status = status.clone();

        Callback::from(move |_| {
            loading.set(true);
            let train_id = train.id.clone();
            let station_id = station_id.clone();
            let loading = loading.clone();
            let current_location = current_location.clone();
            let time_to_station = time_to_station.clone();
            let status = status.clone();
            let train_handle = train.clone();

            spawn_local(async move {
                let arrivals = fetch_arrivals(&station_id).await;

                if let Some(updated_train) = arrivals.iter().find(|a| a.id == train_id) {
                    current_location.set(updated_train.current_location.clone());
                    time_to_station.set(updated_train.time_to_station);

                    if updated_train.time_to_station < 30 {
                        status.set("Arriving".to_string());
                    } else if updated_train.time_to_station < 60 {
                        status.set("Approaching".to_string());
                    } else {
                        status.set("En Route".to_string());
                    }

                    train_handle.set(updated_train.clone());
                } else if *time_to_station <= 0 {
                    status.set("Arrived".to_string());
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

            track_interval.set(Some(interval));
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

    let update_train_status = {
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
                    <p class="font-medium">{ &train.towards }</p>
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
                    <p class="text-sm text-gray-500">{"Expected Arrival"}</p>
                    <p class="font-medium">{ format_time(*time_to_station) }</p>
                </div>
            </div>

            <div class="flex justify-between items-center">
                <div class={format!("px-3 py-1 rounded-full text-sm font-medium {}", status_color)}>
                    { &*status }
                </div>

                <button
                    onclick={update_train_status}
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
