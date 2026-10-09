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
    pub destination_name: Option<String>,
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
    #[serde(rename = "expectedArrival", default)]
    pub expected_arrival: String,
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
    let mut resp = Request::get("/stop_points.json")
        .send()
        .await
        .expect("Failed to fetch stations")
        .json::<Vec<StopPoint>>()
        .await
        .expect("Failed to parse station data");

    resp.iter_mut().for_each(|station| {
        // Normalize station names
        station.common_name = normalize_station_name(&station.common_name);
    });

    resp
}

pub async fn fetch_arrivals(station_ids: &[String]) -> Vec<Arrival> {
    let mut all_arrivals = Vec::new();

    for station_id in station_ids {
        let url = format!("https://api.tfl.gov.uk/StopPoint/{}/Arrivals", station_id);

        let mut resp = Request::get(&url)
            .send()
            .await
            .expect("Failed to fetch arrivals")
            .json::<Vec<Arrival>>()
            .await
            .expect("Failed to parse arrival data");

        // Normalize destination names
        for arrival in &mut resp {
            if let Some(dest) = &arrival.destination_name {
                arrival.destination_name = Some(normalize_station_name(dest));
            }
        }

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

    // Sort first so deduplication keeps the earliest prediction for each train
    let mut sorted: Vec<&Arrival> = arrivals.iter().collect();
    sorted.sort_by_key(|a| a.time_to_station);

    // The same train can be reported by several stop IDs of a station group.
    // Vehicle IDs are only unique within a line, so the line is part of the key.
    let mut seen = std::collections::HashSet::new();
    let mut deduped_arrivals: Vec<&Arrival> = Vec::new();

    for arrival in sorted {
        let line = arrival.line_id.as_deref().unwrap_or(&arrival.line_name);
        let key = match &arrival.vehicle_id {
            Some(vehicle_id) => format!("{line}|{vehicle_id}"),
            None => format!(
                "{line}|{}|{}|{}",
                arrival.platform_name,
                arrival.destination_name.as_deref().unwrap_or_default(),
                arrival.expected_arrival
            ),
        };

        if seen.insert(key) {
            deduped_arrivals.push(arrival);
        }
    }

    // Now group the deduplicated arrivals
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
    // Remove various suffix patterns recursively
    let suffixes = [
        " Underground Station",
        " DLR Station",
        " Rail Station",
        " Station",
        " (London)",
        " (Berks)",
        " (H&C Line)-Underground",
        " (for ExCel)",
        " (for Maritime Greenwich)",
    ];

    let mut normalized = name.to_string();
    'outer: loop {
        for suffix in &suffixes {
            if let Some(stripped) = normalized.strip_suffix(suffix) {
                normalized = stripped.to_string();
                continue 'outer;
            }
        }
        break;
    }

    // Remove "London " prefix
    normalized = normalized
        .strip_prefix("London ")
        .unwrap_or(&normalized)
        .to_string();

    // Special cases mapping
    match normalized.as_str() {
        // Multiple line stations
        "Hammersmith (Dist&Picc Line)" | "Hammersmith (H&C Line)" => "Hammersmith".to_string(),
        "Edgware Road (Bakerloo)" | "Edgware Road (Circle Line)" => "Edgware Road".to_string(),
        "Shepherds Bush" | "Shepherd's Bush (Central)" => "Shepherd's Bush".to_string(),

        // Stations with variant spellings
        "Queens Park" | "Queen's Park" => "Queen's Park".to_string(),

        // Richmond variants
        "Richmond" => "Richmond".to_string(),

        _ => normalized,
    }
}

pub fn group_stations_by_name(stations: &[StopPoint]) -> Vec<GroupedStation> {
    let mut grouped: HashMap<String, Vec<StopPoint>> = HashMap::new();

    // Group stations by normalized common name
    for station in stations {
        grouped
            .entry(station.common_name.clone())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn arrival(id: &str, line: &str, vehicle: Option<&str>, platform: &str, secs: i32) -> Arrival {
        Arrival {
            id: id.to_string(),
            platform_name: platform.to_string(),
            destination_name: Some("Somewhere".to_string()),
            current_location: String::new(),
            time_to_station: secs,
            vehicle_id: vehicle.map(str::to_string),
            line_id: Some(line.to_lowercase()),
            line_name: line.to_string(),
            mode_name: "tube".to_string(),
            expected_arrival: format!("2026-01-01T00:00:{secs:02}Z"),
        }
    }

    fn count(grouped: &[(String, Vec<(String, Vec<Arrival>)>)]) -> usize {
        grouped
            .iter()
            .flat_map(|(_, platforms)| platforms.iter().map(|(_, a)| a.len()))
            .sum()
    }

    #[test]
    fn same_vehicle_id_on_different_lines_is_kept() {
        let arrivals = [
            arrival("1", "Bakerloo", Some("210"), "Northbound", 60),
            arrival("2", "Victoria", Some("210"), "Southbound", 120),
        ];
        assert_eq!(count(&group_arrivals_by_line_platform(&arrivals)), 2);
    }

    #[test]
    fn duplicate_vehicle_on_same_line_keeps_earliest() {
        let arrivals = [
            arrival("1", "Central", Some("5"), "Eastbound", 300),
            arrival("2", "Central", Some("5"), "Eastbound", 240),
        ];
        let grouped = group_arrivals_by_line_platform(&arrivals);
        assert_eq!(count(&grouped), 1);
        assert_eq!(grouped[0].1[0].1[0].id, "2");
    }

    #[test]
    fn trains_without_vehicle_id_to_same_destination_are_kept() {
        let arrivals = [
            arrival("1", "Central", None, "Eastbound", 60),
            arrival("2", "Central", None, "Eastbound", 180),
        ];
        assert_eq!(count(&group_arrivals_by_line_platform(&arrivals)), 2);
    }

    #[test]
    fn groups_are_sorted_by_line_platform_and_time() {
        let arrivals = [
            arrival("1", "Victoria", Some("1"), "Southbound", 200),
            arrival("2", "Bakerloo", Some("2"), "Northbound", 100),
            arrival("3", "Victoria", Some("3"), "Northbound", 50),
            arrival("4", "Victoria", Some("4"), "Southbound", 20),
        ];
        let grouped = group_arrivals_by_line_platform(&arrivals);
        assert_eq!(grouped[0].0, "Bakerloo");
        assert_eq!(grouped[1].1[0].0, "Northbound");
        let southbound: Vec<_> = grouped[1].1[1].1.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(southbound, ["4", "1"]);
    }
}
