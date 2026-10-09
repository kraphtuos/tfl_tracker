use crate::api::{ApiError, Arrival, fetch_stations};
use crate::components::arrivals_board::ArrivalsBoard;
use crate::components::spinner::LoadingSpinner;
use crate::components::station_search::StationSearch;
use crate::components::train_tracker::{TrackedTrain, TrainTracker};
use crate::stations::Station;
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
    Track(TrackedTrain),
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
                trains.push(tracked);
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
    let tracked = use_reducer(TrackedTrains::default);
    let collapsed_lines = use_state(|| Rc::new(HashSet::<String>::new()));

    let load_stations = {
        let stations = stations.clone();
        Callback::from(move |()| {
            stations.set(StationsState::Loading);
            let stations = stations.clone();
            spawn_local(async move {
                stations.set(match fetch_stations().await {
                    Ok(list) => StationsState::Loaded(Rc::new(list)),
                    Err(e) => StationsState::Failed(e),
                });
            });
        })
    };

    {
        let load_stations = load_stations.clone();
        use_effect_with((), move |_| load_stations.emit(()));
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
            tracked.dispatch(TrackedAction::Track(TrackedTrain {
                train: arrival,
                stop_id,
                collapsed: false,
            }));
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
        <div class="container mx-auto p-2 sm:p-4">
            <h1 class="text-xl sm:text-2xl font-bold mb-4">{ "TfL Tracker" }</h1>

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
        </div>
    }
}
