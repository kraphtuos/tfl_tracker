use crate::stations::{Station, StopPoint, group_stations_by_name, normalize_station_name};
use futures::future::try_join_all;
use gloo_net::http::Request;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;

const API_BASE: &str = "https://api.tfl.gov.uk";

#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    Network(String),
    Status(u16),
    Parse(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Network(_) => {
                write!(
                    f,
                    "Couldn't reach TfL. Check your connection and try again."
                )
            }
            ApiError::Status(429) => {
                write!(
                    f,
                    "TfL is limiting requests right now. Try again in a minute."
                )
            }
            ApiError::Status(code) => write!(f, "TfL returned an error (HTTP {code})."),
            ApiError::Parse(_) => write!(f, "Received data in an unexpected format."),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Arrival {
    pub id: String,
    /// Stop ID the prediction is for
    #[serde(default)]
    pub naptan_id: String,
    #[serde(default)]
    pub platform_name: String,
    pub destination_name: Option<String>,
    #[serde(default)]
    pub current_location: String,
    pub time_to_station: i32,
    pub vehicle_id: Option<String>,
    pub line_id: Option<String>,
    pub line_name: String,
    pub mode_name: String,
    #[serde(default)]
    pub expected_arrival: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VehicleArrival {
    pub line_id: String,
    pub station_name: String,
    #[serde(default)]
    pub current_location: String,
    pub time_to_station: i32,
}

async fn get_json<T: DeserializeOwned>(url: &str) -> Result<T, ApiError> {
    let result = async {
        let response = Request::get(url)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;
        if !response.ok() {
            return Err(ApiError::Status(response.status()));
        }
        response
            .json::<T>()
            .await
            .map_err(|e| ApiError::Parse(e.to_string()))
    }
    .await;

    if let Err(e) = &result {
        web_sys::console::warn_1(&format!("Request to {url} failed: {e:?}").into());
    }
    result
}

pub async fn fetch_stations() -> Result<Vec<Station>, ApiError> {
    // Relative so the app also works when served from a sub-path
    let stop_points: Vec<StopPoint> = get_json("stop_points.json").await?;
    Ok(group_stations_by_name(&stop_points))
}

/// Arrivals for all the given stop IDs, fetched in parallel.
pub async fn fetch_arrivals(stop_ids: &[String]) -> Result<Vec<Arrival>, ApiError> {
    let urls: Vec<String> = stop_ids
        .iter()
        .map(|id| format!("{API_BASE}/StopPoint/{id}/Arrivals"))
        .collect();
    let requests = urls.iter().map(|url| get_json::<Vec<Arrival>>(url));
    let mut arrivals: Vec<Arrival> = try_join_all(requests)
        .await?
        .into_iter()
        .flatten()
        .collect();

    for arrival in &mut arrivals {
        if let Some(dest) = &arrival.destination_name {
            arrival.destination_name = Some(normalize_station_name(dest));
        }
    }

    Ok(arrivals)
}

pub async fn fetch_vehicle_arrivals(vehicle_id: &str) -> Result<Vec<VehicleArrival>, ApiError> {
    let mut arrivals: Vec<VehicleArrival> =
        get_json(&format!("{API_BASE}/Vehicle/{vehicle_id}/Arrivals")).await?;

    for arrival in &mut arrivals {
        arrival.station_name = normalize_station_name(&arrival.station_name);
    }

    Ok(arrivals)
}
