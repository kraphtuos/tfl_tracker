use crate::app::App;

mod api;
mod app;
mod arrivals;
mod components;
mod stations;
mod storage;
mod utils;

fn main() {
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}
