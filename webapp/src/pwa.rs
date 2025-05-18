use wasm_bindgen::prelude::*;
use web_sys::{HtmlElement, window};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

pub fn init_pwa() {
    // Check if running in a browser
    if web_sys::window().is_none() {
        return;
    }

    // Show installation button if browser supports PWA installation
    show_install_button();
}

fn show_install_button() {
    if let Some(window) = window() {
        let document = window.document().expect("Missing document");

        // Check if installation container exists
        if let Some(install_container) = document
            .get_element_by_id("install-container")
            .and_then(|el| el.dyn_into::<HtmlElement>().ok())
        {
            // Add handler to show install button when beforeinstallprompt occurs
            let closure = Closure::wrap(Box::new(move || {
                install_container
                    .style()
                    .set_property("display", "block")
                    .unwrap();
            }) as Box<dyn FnMut()>);

            // Register the event handler
            if let Err(e) = window.add_event_listener_with_callback(
                "beforeinstallprompt",
                closure.as_ref().unchecked_ref(),
            ) {
                log(&format!("Error adding event listener: {:?}", e));
            }

            // Leak the closure to keep it alive for the life of the page
            closure.forget();
        }
    }
}
