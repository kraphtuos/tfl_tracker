mod app;
mod station;
mod train_tracker;

use app::App;

fn main() {
    yew::Renderer::<App>::new().render();
}
