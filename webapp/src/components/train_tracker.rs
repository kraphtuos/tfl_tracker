use crate::api::{Arrival, fetch_arrivals, fetch_vehicle_arrivals};
use crate::utils::{format_arrival_time, get_status_color, line_theme, page_visible};
use gloo_timers::callback::Interval;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

/// How often a tracked train's position is refreshed
const POLL_INTERVAL_MS: u32 = 15_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackedTrain {
    pub train: Arrival,
    /// Stop ID the arrival was predicted for
    pub stop_id: String,
    #[serde(default)]
    pub collapsed: bool,
    /// When tracking started, in milliseconds since the epoch
    pub tracked_at: f64,
}

#[derive(Properties, PartialEq)]
pub struct TrainTrackerProps {
    pub tracked: TrackedTrain,
    pub on_close: Callback<()>,
    pub on_toggle: Callback<()>,
}

#[derive(Clone, Debug, PartialEq)]
struct NextStop {
    station_name: String,
    time_to_station: i32,
}

#[derive(Clone, PartialEq)]
struct TrainStatus {
    current_location: String,
    time_to_station: i32,
    status: &'static str,
    next_stops: Vec<NextStop>,
}

impl TrainStatus {
    fn from_arrival(arrival: &Arrival) -> Self {
        Self {
            current_location: arrival.current_location.clone(),
            time_to_station: arrival.time_to_station,
            status: "Approaching",
            next_stops: Vec::new(),
        }
    }
}

/// Identifiers needed for the vehicle API; the vehicle ID may only become known later.
struct TrainIds {
    vehicle_id: Option<String>,
    line_id: Option<String>,
}

async fn poll_status(
    ids: Rc<RefCell<TrainIds>>,
    train_id: String,
    stop_id: String,
) -> Result<TrainStatus, String> {
    // Without a vehicle ID, look the train up in its station's arrivals
    if ids.borrow().vehicle_id.is_none() {
        let arrivals = fetch_arrivals(&[stop_id])
            .await
            .map_err(|e| e.to_string())?;
        let arrival = arrivals
            .iter()
            .find(|a| a.id == train_id)
            .ok_or("Train no longer found at this station")?;
        match &arrival.vehicle_id {
            Some(vehicle_id) => {
                let mut ids = ids.borrow_mut();
                ids.vehicle_id = Some(vehicle_id.clone());
                ids.line_id = arrival.line_id.clone().or(ids.line_id.take());
            }
            // No live tracking available; show what the station knows
            None => return Ok(TrainStatus::from_arrival(arrival)),
        }
    }

    let (vehicle_id, line_id) = {
        let ids = ids.borrow();
        (
            ids.vehicle_id.clone().unwrap_or_default(),
            ids.line_id.clone().unwrap_or_default(),
        )
    };

    // Vehicle IDs are only unique within a line
    let mut predictions: Vec<_> = fetch_vehicle_arrivals(&vehicle_id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|a| a.line_id == line_id)
        .collect();
    predictions.sort_by_key(|a| a.time_to_station);

    let next = predictions
        .first()
        .ok_or("No predictions found for this train")?;

    Ok(TrainStatus {
        current_location: next.current_location.clone(),
        time_to_station: next.time_to_station,
        status: match next.time_to_station {
            ..=30 => "Arrived",
            31..=60 => "Arriving",
            _ => "Approaching",
        },
        next_stops: predictions
            .iter()
            .take(5)
            .map(|a| NextStop {
                station_name: a.station_name.clone(),
                time_to_station: a.time_to_station,
            })
            .collect(),
    })
}

#[function_component(TrainTracker)]
pub fn train_tracker(props: &TrainTrackerProps) -> Html {
    let train = &props.tracked.train;
    let show_train_id = use_state(|| false);
    let status = use_state(|| TrainStatus::from_arrival(train));
    let loading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let in_flight = use_mut_ref(|| false);
    let ids = use_mut_ref(|| TrainIds {
        vehicle_id: train.vehicle_id.clone(),
        line_id: train.line_id.clone(),
    });

    let refresh = {
        let status = status.clone();
        let loading = loading.clone();
        let error = error.clone();
        let train_id = train.id.clone();
        let stop_id = props.tracked.stop_id.clone();
        let ids = ids.clone();
        Callback::from(move |()| {
            if in_flight.replace(true) {
                return;
            }
            loading.set(true);
            let status = status.clone();
            let loading = loading.clone();
            let error = error.clone();
            let in_flight = in_flight.clone();
            let future = poll_status(ids.clone(), train_id.clone(), stop_id.clone());
            spawn_local(async move {
                match future.await {
                    Ok(new_status) => {
                        status.set(new_status);
                        error.set(None);
                    }
                    Err(message) => error.set(Some(message)),
                }
                *in_flight.borrow_mut() = false;
                loading.set(false);
            });
        })
    };

    // Poll while mounted and visible; dropping the interval on unmount cancels it
    {
        let refresh = refresh.clone();
        use_effect_with((), move |_| {
            refresh.emit(());
            let interval = Interval::new(POLL_INTERVAL_MS, move || {
                if page_visible() {
                    refresh.emit(());
                }
            });
            move || drop(interval)
        });
    }

    let theme = line_theme(&train.line_name);
    let destination = train.destination_name.as_deref().unwrap_or("Unknown");
    let is_collapsed = props.tracked.collapsed;

    let toggle_train_id = {
        let show_train_id = show_train_id.clone();
        Callback::from(move |_: MouseEvent| show_train_id.set(!*show_train_id))
    };

    html! {
        <div class="rounded-lg shadow-md overflow-hidden">
            <div class={classes!("flex", "items-center", "border-b", theme.border, theme.light)}>
                <button
                    class="flex-grow flex items-center justify-between gap-2 p-4 text-left min-w-0"
                    onclick={props.on_toggle.reform(|_: MouseEvent| ())}
                    aria-expanded={(!is_collapsed).to_string()}
                >
                    <span class="flex items-center gap-2 min-w-0">
                        <span class="font-medium truncate">{ destination }</span>
                        <span class="text-sm text-gray-500 truncate">{ &status.current_location }</span>
                    </span>
                    <span aria-hidden="true">{ if is_collapsed { "▼" } else { "▲" } }</span>
                </button>
                <button
                    onclick={props.on_close.reform(|_: MouseEvent| ())}
                    class="mr-3 p-1 text-gray-400 hover:text-gray-600"
                    title="Stop tracking"
                    aria-label="Stop tracking"
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
                        <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
                    </svg>
                </button>
            </div>
            if !is_collapsed {
                <div class="p-4">
                    <div class="grid grid-cols-2 gap-4 mb-4">
                        <div>
                            <p class="text-sm text-gray-500">{ "Destination" }</p>
                            <p class="font-medium">{ destination }</p>
                        </div>
                        <div>
                            <p class="text-sm text-gray-500">{ "Platform" }</p>
                            <p class="font-medium">{ &train.platform_name }</p>
                        </div>
                        <div>
                            <p class="text-sm text-gray-500">{ "Current Location" }</p>
                            <p class="font-medium">{ &status.current_location }</p>
                        </div>
                        <div>
                            <p class="text-sm text-gray-500">{ "Next Arrival" }</p>
                            <p class="font-medium">{ format_arrival_time(status.time_to_station) }</p>
                        </div>
                        <div class="col-span-2">
                            <button
                                onclick={toggle_train_id}
                                aria-expanded={show_train_id.to_string()}
                                class="text-blue-600 hover:text-blue-800 text-sm py-1"
                            >
                                { if *show_train_id { "Hide Train ID" } else { "Show Train ID" } }
                            </button>
                            if *show_train_id {
                                <p class="mt-1 text-sm text-gray-600">
                                    { ids.borrow().vehicle_id.clone().unwrap_or_else(|| train.id.clone()) }
                                </p>
                            }
                        </div>
                    </div>

                    if !status.next_stops.is_empty() {
                        <div class="mb-4">
                            <h4 class="font-medium mb-2">{ "Next Stops" }</h4>
                            <div class="bg-gray-50 rounded p-2">
                                <table class="w-full text-sm">
                                    <thead>
                                        <tr class="border-b">
                                            <th class="text-left py-1">{ "Station" }</th>
                                            <th class="text-right py-1">{ "Arrival" }</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        { for status.next_stops.iter().map(|stop| html! {
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

                    if let Some(message) = &*error {
                        <div class="mb-4 p-2 bg-red-50 text-red-700 rounded text-sm" role="alert">
                            { message }
                        </div>
                    }

                    <div class="flex justify-between items-center">
                        <div class={classes!("px-3", "py-1", "rounded-full", "text-sm", "font-medium", get_status_color(status.status))}>
                            { status.status }
                        </div>

                        <button
                            onclick={refresh.reform(|_: MouseEvent| ())}
                            disabled={*loading}
                            class="text-blue-600 hover:text-blue-800 text-sm flex items-center"
                        >
                            if *loading {
                                <div class="mr-1 h-4 w-4 border-t-2 border-b-2 border-blue-600 rounded-full animate-spin" aria-hidden="true"></div>
                                { "Updating..." }
                            } else {
                                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-1" fill="none" viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true">
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
