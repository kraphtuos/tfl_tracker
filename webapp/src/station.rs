use gloo_net::http::Request;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StopPoint {
    pub id: String,
    pub common_name: String,
    pub stop_type: String,
    pub mode_name: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Arrival {
    pub id: String,
    #[serde(rename = "platformName")]
    pub platform_name: String,
    #[serde(rename = "destinationName")]
    pub towards: Option<String>,
    #[serde(rename = "currentLocation")]
    pub current_location: String,
    #[serde(rename = "timeToStation")]
    pub time_to_station: i32,
    #[serde(rename = "vehicleId")]
    pub vehicle_id: Option<String>,
    #[serde(rename = "lineId")]
    pub line_id: Option<String>,
    #[serde(rename = "lineName")]
    pub line_name: String,
    #[serde(rename = "modeName")]
    pub mode_name: String,
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
    // Get stations from the generated JSON file
    let resp = Request::get("/stop_points.json")
        .send()
        .await
        .expect("Failed to fetch stations")
        .json::<Vec<StopPoint>>()
        .await
        .expect("Failed to parse station data");

    resp
}

pub async fn fetch_arrivals(station_ids: &[String]) -> Vec<Arrival> {
    let mut all_arrivals = Vec::new();

    for station_id in station_ids {
        let url = format!("https://api.tfl.gov.uk/StopPoint/{}/Arrivals", station_id);

        let resp = Request::get(&url)
            .send()
            .await
            .expect("Failed to fetch arrivals")
            .json::<Vec<Arrival>>()
            .await
            .expect("Failed to parse arrival data");

        all_arrivals.extend(resp);
    }

    all_arrivals
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
) -> Vec<(String, Vec<(String, Vec<Arrival>)>)> {
    let mut grouped: HashMap<String, HashMap<String, Vec<Arrival>>> = HashMap::new();

    // First, deduplicate arrivals based on vehicle_id and destination
    let mut seen = std::collections::HashSet::new();
    let mut deduped_arrivals: Vec<&Arrival> = Vec::new();

    for arrival in arrivals {
        let key = match (&arrival.vehicle_id, &arrival.towards) {
            (Some(vehicle_id), _) => vehicle_id.clone(), // Use vehicle_id if available
            (None, Some(towards)) => format!(
                "{}-{}-{}",
                arrival.line_name, arrival.platform_name, towards
            ),
            (None, None) => arrival.id.clone(), // Fallback to arrival id
        };

        if seen.insert(key) {
            deduped_arrivals.push(arrival);
        }
    } // Now group the deduplicated arrivals
    for arrival in deduped_arrivals {
        let normalized_line_name = normalize_line_name(&arrival.line_name);
        let line_group = grouped
            .entry(normalized_line_name)
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

    // Collect and sort lines and platforms
    let mut lines: Vec<_> = grouped.into_iter().collect();
    lines.sort_by(|a, b| a.0.cmp(&b.0)); // Sort by line name

    lines
        .into_iter()
        .map(|(line_name, platforms)| {
            let mut platforms_vec: Vec<_> = platforms.into_iter().collect();
            platforms_vec.sort_by(|a, b| a.0.cmp(&b.0)); // Sort by platform name
            (line_name, platforms_vec)
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupedStation {
    pub id: String, // First station ID in the group
    pub common_name: String,
    pub modes: Vec<String>,
    pub all_stations: Vec<StopPoint>,
}

fn normalize_station_name(name: &str) -> String {
    // Remove "(London)" and "(Berks)" suffix
    let name = name
        .replace("(London)", "")
        .replace("(Berks)", "")
        .trim()
        .to_string();

    // Remove "London " prefix
    let name = if name.starts_with("London ") {
        name.strip_prefix("London ").unwrap_or(&name).to_string()
    } else {
        name
    };

    // Special cases mapping
    match name.as_str() {
        // Major rail stations
        "Liverpool Street" | "London Liverpool Street" => "Liverpool Street".to_string(),
        "Paddington" | "Paddington (H&C Line)-Underground" | "London Paddington" => {
            "Paddington".to_string()
        }
        "Euston" | "London Euston" => "Euston".to_string(),

        // Elizabeth line stations
        "Custom House" | "Custom House (for ExCel)" => "Custom House".to_string(),

        // Multiple line stations
        "Hammersmith (Dist&Picc Line)" | "Hammersmith (H&C Line)" => "Hammersmith".to_string(),
        "Edgware Road (Bakerloo)" | "Edgware Road (Circle Line)" => "Edgware Road".to_string(),
        "Shepherds Bush" | "Shepherd's Bush (Central)" => "Shepherd's Bush".to_string(),

        // Stations with variant spellings
        "Queens Park (London)" | "Queen's Park" => "Queen's Park".to_string(),

        _ => name,
    }
}

pub fn group_stations_by_name(stations: &[StopPoint]) -> Vec<GroupedStation> {
    let mut grouped: HashMap<String, Vec<StopPoint>> = HashMap::new();

    // Group stations by normalized common name
    for station in stations {
        let normalized_name = normalize_station_name(&station.common_name);
        grouped
            .entry(normalized_name)
            .or_default()
            .push(station.clone());
    }

    // Convert to GroupedStation structs
    let mut result: Vec<GroupedStation> = grouped
        .into_iter()
        .map(|(name, stations)| {
            let modes: Vec<String> = stations
                .iter()
                .map(|s| s.mode_name.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            GroupedStation {
                id: stations[0].id.clone(),
                common_name: name,
                modes,
                all_stations: stations,
            }
        })
        .collect();

    // Sort by station name
    result.sort_by(|a, b| a.common_name.cmp(&b.common_name));
    result
}

pub fn normalize_line_name(line_name: &str) -> String {
    match line_name {
        "Elizabeth line" => "Elizabeth".to_string(),
        _ => line_name.to_string(),
    }
}
