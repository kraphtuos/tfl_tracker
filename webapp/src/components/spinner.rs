use yew::prelude::*;

#[function_component(LoadingSpinner)]
pub fn loading_spinner() -> Html {
    html! {
        <div class="inline-block animate-spin rounded-full h-5 w-5 border-t-2 border-b-2 border-blue-500" aria-hidden="true"></div>
    }
}
