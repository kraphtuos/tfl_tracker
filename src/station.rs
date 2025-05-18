use gloo_net::http::Request;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StopPoint {
    pub id: String,
    #[serde(rename = "commonName")]
    pub common_name: String,
    #[serde(rename = "stopType")]
    pub stop_type: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Arrival {
    pub id: String,
    #[serde(rename = "lineName")]
    pub line_name: String,
    #[serde(rename = "platformName")]
    pub platform_name: String,
    pub towards: String,
    #[serde(rename = "currentLocation")]
    pub current_location: String,
    #[serde(rename = "timeToStation")]
    pub time_to_station: i32,
    #[serde(rename = "vehicleId")]
    pub vehicle_id: Option<String>,
    #[serde(rename = "lineId")]
    pub line_id: Option<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct VehicleArrival {
    pub id: String,
    #[serde(rename = "lineId")]
    pub line_id: String,
    #[serde(rename = "lineName")]
    pub line_name: String,
    #[serde(rename = "platformName")]
    pub platform_name: Option<String>,
    #[serde(rename = "stationName")]
    pub station_name: String,
    #[serde(rename = "towards")]
    pub towards: String,
    #[serde(rename = "currentLocation")]
    pub current_location: String,
    #[serde(rename = "timeToStation")]
    pub time_to_station: i32,
}

pub async fn fetch_tube_stations() -> Vec<StopPoint> {
    let resp: serde_json::Value = Request::get("https://api.tfl.gov.uk/StopPoint/Mode/tube")
        .send()
        .await
        .expect("Failed to fetch stations")
        .json()
        .await
        .expect("Failed to parse JSON");

    let all: Vec<StopPoint> =
        serde_json::from_value(resp["stopPoints"].clone()).expect("Failed to deserialize stations");

    let mut res: Vec<StopPoint> = all
        .into_iter()
        .filter(|s| s.stop_type == "NaptanMetroStation")
        .collect();

    res.sort_by(|a, b| a.common_name.cmp(&b.common_name));
    res
}

pub async fn fetch_arrivals(station_id: &str) -> Vec<Arrival> {
    let url = format!("https://api.tfl.gov.uk/StopPoint/{}/Arrivals", station_id);

    let resp = Request::get(&url)
        .send()
        .await
        .expect("Failed to fetch arrivals")
        .json::<Vec<Arrival>>()
        .await
        .expect("Failed to parse arrival data");

    resp
}

pub async fn fetch_vehicle_arrivals(vehicle_id: &str) -> Vec<VehicleArrival> {
    let url = format!("https://api.tfl.gov.uk/Vehicle/{}/Arrivals", vehicle_id);

    let resp = Request::get(&url)
        .send()
        .await
        .expect("Failed to fetch vehicle arrivals")
        .json::<Vec<VehicleArrival>>()
        .await
        .expect("Failed to parse vehicle arrival data");

    resp
}

// Helper function to group arrivals by line and platform
pub fn group_arrivals_by_line_platform(
    arrivals: &[Arrival],
) -> HashMap<String, HashMap<String, Vec<Arrival>>> {
    let mut grouped: HashMap<String, HashMap<String, Vec<Arrival>>> = HashMap::new();

    for arrival in arrivals {
        let line_group = grouped
            .entry(arrival.line_name.clone())
            .or_insert_with(HashMap::new);

        let platform_group = line_group
            .entry(arrival.platform_name.clone())
            .or_insert_with(Vec::new);

        platform_group.push(arrival.clone());
    }

    // Sort each group by time to station
    for (_, platforms) in grouped.iter_mut() {
        for (_, arrivals) in platforms.iter_mut() {
            arrivals.sort_by(|a, b| a.time_to_station.cmp(&b.time_to_station));
        }
    }

    grouped
}
