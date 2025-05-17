mod app;
mod station;

use app::App;

fn main() {
    yew::Renderer::<App>::new().render();
}
