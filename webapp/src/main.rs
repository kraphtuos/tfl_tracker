use crate::app::App;

mod api;
mod app;
mod arrivals;
mod components;
mod pwa;
mod stations;
mod utils;

fn main() {
    console_error_panic_hook::set_once();
    pwa::init_pwa();
    yew::Renderer::<App>::new().render();
}
