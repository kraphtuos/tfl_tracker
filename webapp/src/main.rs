use crate::app::App;

mod app;
mod pwa;
mod station;
mod train_tracker;

fn main() {
    pwa::init_pwa();
    yew::Renderer::<App>::new().render();
}
