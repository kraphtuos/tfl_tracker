use crate::stations::Station;
use futures::future::join_all;
use gloo_net::http::Request;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use station_names::normalize_station_name;
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
    get_json("stop_points.json").await
}

#[derive(Debug, PartialEq)]
pub struct StationArrivals {
    pub arrivals: Vec<Arrival>,
    /// Set when some stops failed to load but others succeeded
    pub partial_error: Option<ApiError>,
}

/// Combine per-stop results, failing only if every stop failed.
fn combine_stop_results(
    results: Vec<Result<Vec<Arrival>, ApiError>>,
) -> Result<StationArrivals, ApiError> {
    let mut arrivals = Vec::new();
    let mut first_error = None;
    let mut any_succeeded = false;

    for result in results {
        match result {
            Ok(stop_arrivals) => {
                any_succeeded = true;
                arrivals.extend(stop_arrivals);
            }
            Err(e) => {
                first_error.get_or_insert(e);
            }
        }
    }

    match first_error {
        Some(e) if !any_succeeded => Err(e),
        partial_error => Ok(StationArrivals {
            arrivals,
            partial_error,
        }),
    }
}

/// Arrivals for all the given stop IDs, fetched in parallel. One failing stop
/// (e.g. an ID TfL has since changed) doesn't hide the others.
pub async fn fetch_arrivals(stop_ids: &[String]) -> Result<StationArrivals, ApiError> {
    let urls: Vec<String> = stop_ids
        .iter()
        .map(|id| format!("{API_BASE}/StopPoint/{id}/Arrivals"))
        .collect();
    let requests = urls.iter().map(|url| get_json::<Vec<Arrival>>(url));
    let mut result = combine_stop_results(join_all(requests).await)?;

    for arrival in &mut result.arrivals {
        if let Some(dest) = &arrival.destination_name {
            arrival.destination_name = Some(normalize_station_name(dest));
        }
    }

    Ok(result)
}

pub async fn fetch_vehicle_arrivals(vehicle_id: &str) -> Result<Vec<VehicleArrival>, ApiError> {
    let mut arrivals: Vec<VehicleArrival> =
        get_json(&format!("{API_BASE}/Vehicle/{vehicle_id}/Arrivals")).await?;

    for arrival in &mut arrivals {
        arrival.station_name = normalize_station_name(&arrival.station_name);
    }

    Ok(arrivals)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arrival(id: &str) -> Arrival {
        Arrival {
            id: id.to_string(),
            naptan_id: String::new(),
            platform_name: String::new(),
            destination_name: None,
            current_location: String::new(),
            time_to_station: 0,
            vehicle_id: None,
            line_id: None,
            line_name: "Central".to_string(),
            mode_name: "tube".to_string(),
            expected_arrival: String::new(),
        }
    }

    #[test]
    fn one_failing_stop_keeps_the_others() {
        let result = combine_stop_results(vec![
            Ok(vec![arrival("1")]),
            Err(ApiError::Status(404)),
            Ok(vec![arrival("2")]),
        ])
        .unwrap();
        assert_eq!(result.arrivals.len(), 2);
        assert_eq!(result.partial_error, Some(ApiError::Status(404)));
    }

    #[test]
    fn all_stops_failing_is_an_error() {
        let result =
            combine_stop_results(vec![Err(ApiError::Status(429)), Err(ApiError::Status(500))]);
        assert_eq!(result, Err(ApiError::Status(429)));
    }

    #[test]
    fn all_stops_succeeding_has_no_error() {
        let result = combine_stop_results(vec![Ok(vec![]), Ok(vec![arrival("1")])]).unwrap();
        assert_eq!(result.partial_error, None);
        assert_eq!(result.arrivals.len(), 1);
    }
}
