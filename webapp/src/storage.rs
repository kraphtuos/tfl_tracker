//! Persistence of tracked trains in localStorage and of the selected station in the URL.

use crate::components::train_tracker::TrackedTrain;
use crate::stations::Station;
use crate::utils::now_ms;
use wasm_bindgen::JsValue;

const TRACKED_TRAINS_KEY: &str = "tfl-tracker:tracked-trains";

/// Tracked trains older than this are dropped when the app loads
const MAX_TRACKED_AGE_MS: f64 = 2.0 * 60.0 * 60.0 * 1000.0;

/// localStorage, if the browser allows access to it
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub fn load_tracked_trains() -> Vec<TrackedTrain> {
    let Some(json) = local_storage().and_then(|s| s.get_item(TRACKED_TRAINS_KEY).ok()?) else {
        return Vec::new();
    };
    let trains: Vec<TrackedTrain> = serde_json::from_str(&json).unwrap_or_default();
    let now = now_ms();
    trains
        .into_iter()
        .filter(|t| now - t.tracked_at < MAX_TRACKED_AGE_MS)
        .collect()
}

pub fn save_tracked_trains(trains: &[TrackedTrain]) {
    let Some(storage) = local_storage() else {
        return;
    };
    // Storage may be full or blocked; persistence is best-effort
    let _ = if trains.is_empty() {
        storage.remove_item(TRACKED_TRAINS_KEY)
    } else {
        match serde_json::to_string(trains) {
            Ok(json) => storage.set_item(TRACKED_TRAINS_KEY, &json),
            Err(_) => Ok(()),
        }
    };
}

/// The station named by the URL hash (e.g. `#HUBBAN` or `#940GZZLUOXC`).
pub fn station_from_url(stations: &[Station]) -> Option<Station> {
    let hash = web_sys::window()?.location().hash().ok()?;
    let id = hash.trim_start_matches('#');
    if id.is_empty() {
        return None;
    }
    stations.iter().find(|s| s.has_id(id)).cloned()
}

/// Put the selected station in the URL hash without adding a history entry.
pub fn set_station_in_url(station_id: Option<&str>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let location = window.location();
    let wanted = station_id.map(|id| format!("#{id}")).unwrap_or_default();
    if location.hash().unwrap_or_default() == wanted {
        return;
    }
    let url = if wanted.is_empty() {
        // Drop the hash entirely rather than leaving a bare "#"
        format!(
            "{}{}",
            location.pathname().unwrap_or_default(),
            location.search().unwrap_or_default()
        )
    } else {
        wanted
    };
    if let Ok(history) = window.history() {
        let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&url));
    }
}
