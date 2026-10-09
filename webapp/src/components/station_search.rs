use crate::stations::{Station, search_stations};
use std::rc::Rc;
use web_sys::{Element, HtmlInputElement, KeyboardEvent};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct StationSearchProps {
    pub stations: Rc<Vec<Station>>,
    pub selected: Option<Station>,
    pub on_select: Callback<Option<Station>>,
}

#[derive(Clone, Default, PartialEq)]
struct SearchState {
    query: String,
    /// Index of the keyboard-highlighted suggestion
    focused: Option<usize>,
    open: bool,
}

enum SearchAction {
    Input(String),
    Open,
    /// Close the suggestions, putting the selected station's name back if there is one
    Close(Option<String>),
    /// Move the highlight down (`true`) or up within `len` suggestions
    Move {
        down: bool,
        len: usize,
    },
    /// Show `query` with the suggestions closed
    Set(String),
}

impl Reducible for SearchState {
    type Action = SearchAction;

    fn reduce(self: Rc<Self>, action: SearchAction) -> Rc<Self> {
        let mut state = (*self).clone();
        match action {
            SearchAction::Input(query) => {
                state.query = query;
                state.focused = None;
                state.open = true;
            }
            SearchAction::Open => state.open = true,
            SearchAction::Close(restore) => {
                state.open = false;
                state.focused = None;
                if let Some(name) = restore {
                    state.query = name;
                }
            }
            SearchAction::Move { down, len } => {
                state.open = true;
                state.focused = match (state.focused, down) {
                    _ if len == 0 => None,
                    (None, true) => Some(0),
                    (Some(i), true) => Some((i + 1).min(len - 1)),
                    (None, false) | (Some(0), false) => None,
                    (Some(i), false) => Some(i - 1),
                };
            }
            SearchAction::Set(query) => {
                state.query = query;
                state.focused = None;
                state.open = false;
            }
        }
        Rc::new(state)
    }
}

#[function_component(StationSearch)]
pub fn station_search(props: &StationSearchProps) -> Html {
    let state = use_reducer(SearchState::default);
    let list_ref = use_node_ref();

    let results: Vec<Station> = search_stations(&props.stations, &state.query)
        .into_iter()
        .cloned()
        .collect();

    // Show the selected station's name when it changes (e.g. restored from the URL)
    {
        let state = state.clone();
        let selected_name = props.selected.as_ref().map(|s| s.name.clone());
        use_effect_with(selected_name, move |name| {
            if let Some(name) = name {
                state.dispatch(SearchAction::Set(name.clone()));
            }
        });
    }

    // Keep the highlighted suggestion visible when moving with the keyboard
    {
        let list_ref = list_ref.clone();
        use_effect_with(state.focused, move |focused| {
            if let (Some(index), Some(list)) = (focused, list_ref.cast::<Element>())
                && let Some(item) = list.children().item(*index as u32)
            {
                item.scroll_into_view_with_bool(false);
            }
        });
    }

    let select = {
        let state = state.clone();
        let on_select = props.on_select.clone();
        Callback::from(move |station: Station| {
            state.dispatch(SearchAction::Set(station.name.clone()));
            on_select.emit(Some(station));
        })
    };

    let on_input = {
        let state = state.clone();
        let selected = props.selected.clone();
        let on_select = props.on_select.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let query = input.value();
            // Editing the name of the selected station deselects it
            if let Some(station) = &selected
                && station.name.to_lowercase() != query.to_lowercase()
            {
                on_select.emit(None);
            }
            state.dispatch(SearchAction::Input(query));
        })
    };

    let on_focus = {
        let state = state.clone();
        Callback::from(move |_: FocusEvent| state.dispatch(SearchAction::Open))
    };

    let on_blur = {
        let state = state.clone();
        let selected_name = props.selected.as_ref().map(|s| s.name.clone());
        Callback::from(move |_: FocusEvent| {
            // Delay so a tap on a suggestion can complete first
            let state = state.clone();
            let selected_name = selected_name.clone();
            gloo_timers::callback::Timeout::new(200, move || {
                state.dispatch(SearchAction::Close(selected_name));
            })
            .forget();
        })
    };

    let on_keydown = {
        let state = state.clone();
        let results = results.clone();
        let select = select.clone();
        let selected_name = props.selected.as_ref().map(|s| s.name.clone());
        Callback::from(move |e: KeyboardEvent| match e.key().as_str() {
            "ArrowDown" | "ArrowUp" => {
                e.prevent_default();
                state.dispatch(SearchAction::Move {
                    down: e.key() == "ArrowDown",
                    len: results.len(),
                });
            }
            "Enter" => {
                // Pick the highlighted suggestion, or the best match if none is highlighted
                let index = state.focused.unwrap_or(0);
                if state.open
                    && let Some(station) = results.get(index)
                {
                    select.emit(station.clone());
                }
            }
            "Escape" => state.dispatch(SearchAction::Close(selected_name.clone())),
            _ => {}
        })
    };

    let clear_input = {
        let state = state.clone();
        let on_select = props.on_select.clone();
        Callback::from(move |_: MouseEvent| {
            state.dispatch(SearchAction::Set(String::new()));
            on_select.emit(None);
        })
    };

    let prevent_blur = Callback::from(|e: MouseEvent| e.prevent_default());

    html! {
        <div class="relative w-full">
            <div class="relative">
                <input
                    type="text"
                    id="station-search"
                    placeholder="Search for a station..."
                    autocomplete="off"
                    role="combobox"
                    aria-expanded={(state.open && !results.is_empty()).to_string()}
                    aria-controls="station-suggestions"
                    class="w-full p-3 text-base sm:text-lg border border-gray-200 rounded-lg bg-white text-gray-900 focus:ring-2 focus:ring-blue-500 focus:border-transparent pr-10"
                    value={state.query.clone()}
                    oninput={on_input}
                    onfocus={on_focus}
                    onblur={on_blur}
                    onkeydown={on_keydown}
                />
                if !state.query.is_empty() {
                    <button
                        onclick={clear_input}
                        onmousedown={prevent_blur.clone()}
                        class="absolute right-2 top-1/2 -translate-y-1/2 p-2 text-gray-500 hover:text-gray-700"
                        aria-label="Clear search"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
                            <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd" />
                        </svg>
                    </button>
                }
            </div>
            if state.open && !results.is_empty() {
                <div
                    ref={list_ref}
                    id="station-suggestions"
                    role="listbox"
                    class="absolute z-50 w-full mt-1 bg-white border border-gray-200 rounded-lg shadow-lg max-h-[70vh] sm:max-h-60 overflow-y-auto"
                    onmousedown={prevent_blur}
                >
                    { for results.iter().enumerate().map(|(index, station)| {
                        let is_focused = state.focused == Some(index);
                        let onclick = {
                            let station = station.clone();
                            select.reform(move |_: MouseEvent| station.clone())
                        };
                        html! {
                            <div
                                key={station.id.clone()}
                                role="option"
                                aria-selected={is_focused.to_string()}
                                class={classes!(
                                    "p-2",
                                    "cursor-pointer",
                                    "text-gray-900",
                                    "hover:bg-gray-100",
                                    is_focused.then_some("bg-blue-100"),
                                )}
                                {onclick}
                            >
                                <div class="font-medium">{ &station.name }</div>
                                <div class="text-sm text-gray-500">{ station.modes.join(" • ") }</div>
                            </div>
                        }
                    }) }
                </div>
            }
        </div>
    }
}
