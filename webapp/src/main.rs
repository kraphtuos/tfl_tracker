use crate::app::App;

mod app;
mod pwa;
mod station;
mod train_tracker;
mod utils;

fn main() {
    pwa::init_pwa();
    yew::Renderer::<App>::new().render();
}
