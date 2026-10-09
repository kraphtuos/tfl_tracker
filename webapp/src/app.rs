use crate::api::{ApiError, Arrival, fetch_stations};
use crate::components::arrivals_board::ArrivalsBoard;
use crate::components::spinner::LoadingSpinner;
use crate::components::station_search::StationSearch;
use crate::components::train_tracker::{TrackedTrain, TrainTracker};
use crate::stations::Station;
use crate::storage::{
    load_tracked_trains, save_tracked_trains, set_station_in_url, station_from_url,
};
use crate::utils::now_ms;
use gloo_events::EventListener;
use std::collections::HashSet;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
enum StationsState {
    Loading,
    Loaded(Rc<Vec<Station>>),
    Failed(ApiError),
}

#[derive(Clone, Default, PartialEq)]
struct TrackedTrains(Vec<TrackedTrain>);

enum TrackedAction {
    Track(Box<TrackedTrain>),
    Remove(String),
    ToggleCollapsed(String),
}

impl Reducible for TrackedTrains {
    type Action = TrackedAction;

    fn reduce(self: Rc<Self>, action: TrackedAction) -> Rc<Self> {
        let mut trains = self.0.clone();
        match action {
            TrackedAction::Track(tracked) => {
                if trains.iter().any(|t| t.train.id == tracked.train.id) {
                    return self;
                }
                trains.push(*tracked);
            }
            TrackedAction::Remove(id) => trains.retain(|t| t.train.id != id),
            TrackedAction::ToggleCollapsed(id) => {
                if let Some(tracked) = trains.iter_mut().find(|t| t.train.id == id) {
                    tracked.collapsed = !tracked.collapsed;
                }
            }
        }
        Rc::new(Self(trains))
    }
}

#[function_component(App)]
pub fn app() -> Html {
    let stations = use_state(|| StationsState::Loading);
    let selected = use_state(|| None::<Station>);
    let tracked = use_reducer(|| TrackedTrains(load_tracked_trains()));
    let collapsed_lines = use_state(|| Rc::new(HashSet::<String>::new()));

    let load_stations = {
        let stations = stations.clone();
        let selected = selected.clone();
        Callback::from(move |()| {
            stations.set(StationsState::Loading);
            let stations = stations.clone();
            let selected = selected.clone();
            spawn_local(async move {
                stations.set(match fetch_stations().await {
                    Ok(list) => {
                        // Open the station named in the URL, if any
                        if let Some(station) = station_from_url(&list) {
                            selected.set(Some(station));
                        }
                        StationsState::Loaded(Rc::new(list))
                    }
                    Err(e) => StationsState::Failed(e),
                });
            });
        })
    };

    {
        let load_stations = load_stations.clone();
        use_effect_with((), move |_| load_stations.emit(()));
    }

    let loaded_stations = match &*stations {
        StationsState::Loaded(list) => Some(list.clone()),
        _ => None,
    };

    // Follow manual edits of the URL hash
    {
        let selected = selected.clone();
        use_effect_with(loaded_stations.clone(), move |list| {
            let listener = list.clone().and_then(|list| {
                let window = web_sys::window()?;
                Some(EventListener::new(&window, "hashchange", move |_| {
                    selected.set(station_from_url(&list));
                }))
            });
            move || drop(listener)
        });
    }

    // Keep the URL in sync with the selection once stations have loaded
    {
        let selected_id = selected.as_ref().map(|s| s.id.clone());
        use_effect_with((selected_id, loaded_stations.is_some()), |(id, loaded)| {
            if *loaded {
                set_station_in_url(id.as_deref());
            }
        });
    }

    {
        let trains = tracked.0.clone();
        use_effect_with(trains, |trains| save_tracked_trains(trains));
    }

    let on_select = {
        let selected = selected.clone();
        Callback::from(move |station: Option<Station>| selected.set(station))
    };

    let on_track = {
        let tracked = tracked.clone();
        let station = (*selected).clone();
        Callback::from(move |arrival: Arrival| {
            let Some(station) = &station else { return };
            // Remember which of the station's stops the train was predicted for
            let stop_id = if arrival.naptan_id.is_empty() {
                station.stops[0].id.clone()
            } else {
                arrival.naptan_id.clone()
            };
            tracked.dispatch(TrackedAction::Track(Box::new(TrackedTrain {
                train: arrival,
                stop_id,
                collapsed: false,
                tracked_at: now_ms(),
            })));
        })
    };

    let on_toggle_line = {
        let collapsed_lines = collapsed_lines.clone();
        Callback::from(move |line: String| {
            let mut lines = (**collapsed_lines).clone();
            if !lines.remove(&line) {
                lines.insert(line);
            }
            collapsed_lines.set(Rc::new(lines));
        })
    };

    let tracked_ids: Rc<HashSet<String>> =
        Rc::new(tracked.0.iter().map(|t| t.train.id.clone()).collect());

    let content = match &*stations {
        StationsState::Loading => html! {
            <div class="flex items-center justify-center p-4">
                <LoadingSpinner />
                <p class="ml-2">{ "Loading stations..." }</p>
            </div>
        },
        StationsState::Failed(error) => html! {
            <div class="p-4 bg-red-50 text-red-700 rounded" role="alert">
                <p class="mb-2">{ format!("Couldn't load the station list. {error}") }</p>
                <button
                    onclick={load_stations.reform(|_: MouseEvent| ())}
                    class="bg-red-600 hover:bg-red-700 text-white font-bold py-1 px-3 rounded"
                >
                    { "Try again" }
                </button>
            </div>
        },
        StationsState::Loaded(list) => html! {
            <>
                <div class="mb-4 sm:mb-6">
                    <label for="station-search" class="block mb-2 font-medium">{ "Choose a Station" }</label>
                    <StationSearch
                        stations={list.clone()}
                        selected={(*selected).clone()}
                        {on_select}
                    />
                </div>
                if let Some(station) = &*selected {
                    <ArrivalsBoard
                        key={station.id.clone()}
                        station={station.clone()}
                        tracked_ids={tracked_ids.clone()}
                        {on_track}
                        collapsed_lines={(*collapsed_lines).clone()}
                        {on_toggle_line}
                    />
                }
            </>
        },
    };

    html! {
        <>
        // An opaque, full-width sticky header at the top edge lets iOS 26 sample a flat
        // colour there instead of blurring the top of the installed app.
        <header class="ios-app-header sticky top-0 z-40 bg-white">
            <div class="container mx-auto px-2 sm:px-4 pt-2 sm:pt-4 pb-3">
                <h1 class="text-xl sm:text-2xl font-bold">{ "TfL Tracker" }</h1>
            </div>
        </header>

        <main class="container mx-auto px-2 sm:px-4 pt-1 pb-2 sm:pb-4">
            if !tracked.0.is_empty() {
                <div class="mb-6">
                    <h2 class="text-lg font-semibold mb-4">{ "Tracked Trains" }</h2>
                    <div class="space-y-3 sm:space-y-4">
                        { for tracked.0.iter().map(|t| {
                            let id = t.train.id.clone();
                            let on_close = {
                                let tracked = tracked.clone();
                                let id = id.clone();
                                Callback::from(move |()| tracked.dispatch(TrackedAction::Remove(id.clone())))
                            };
                            let on_toggle = {
                                let tracked = tracked.clone();
                                let id = id.clone();
                                Callback::from(move |()| tracked.dispatch(TrackedAction::ToggleCollapsed(id.clone())))
                            };
                            html! {
                                <TrainTracker key={id} tracked={t.clone()} {on_close} {on_toggle} />
                            }
                        }) }
                    </div>
                </div>
            }

            { content }
        </main>
        </>
    }
}
