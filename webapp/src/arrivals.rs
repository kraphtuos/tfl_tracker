use crate::api::Arrival;
use std::collections::{HashMap, HashSet};

/// Arrivals grouped by line, then platform; both sorted by name.
pub type LineArrivals = Vec<(String, Vec<(String, Vec<Arrival>)>)>;

/// Group arrivals by line and platform, removing duplicate predictions for the same train.
pub fn group_arrivals_by_line_platform(arrivals: &[Arrival]) -> LineArrivals {
    let mut grouped: HashMap<String, HashMap<String, Vec<Arrival>>> = HashMap::new();

    // Sort first so deduplication keeps the earliest prediction for each train
    let mut sorted: Vec<&Arrival> = arrivals.iter().collect();
    sorted.sort_by_key(|a| a.time_to_station);

    // The same train can be reported by several stop IDs of a station group.
    // Vehicle IDs are only unique within a line, so the line is part of the key.
    let mut seen = HashSet::new();
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
        let line_group = grouped.entry(normalized_line_name).or_default();

        let platform_group = line_group.entry(arrival.platform_name.clone()).or_default();

        platform_group.push(arrival.clone());
    }

    // Collect and sort lines and platforms
    let mut lines: Vec<_> = grouped.into_iter().collect();
    lines.sort_by(|a, b| a.0.cmp(&b.0));

    lines
        .into_iter()
        .map(|(line_name, platforms)| {
            let mut platforms_vec: Vec<_> = platforms.into_iter().collect();
            platforms_vec.sort_by(|a, b| a.0.cmp(&b.0));
            (line_name, platforms_vec)
        })
        .collect()
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
            naptan_id: String::new(),
            expected_arrival: format!("2026-01-01T00:00:{secs:02}Z"),
        }
    }

    fn count(grouped: &LineArrivals) -> usize {
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
