//! Generates `webapp/stop_points.json`: every tube, Elizabeth line, DLR and
//! Overground station, with all of its stop IDs grouped under one name.

use serde::{Deserialize, Serialize};
use station_names::normalize_station_name;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// TfL mode IDs and the names shown in the app, in display order
const MODES: [(&str, &str); 4] = [
    ("tube", "Underground"),
    ("elizabeth-line", "Elizabeth"),
    ("dlr", "DLR"),
    ("overground", "Overground"),
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModeResponse {
    stop_points: Vec<ApiStopPoint>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiStopPoint {
    id: String,
    common_name: String,
    stop_type: String,
    hub_naptan_code: Option<String>,
}

#[derive(Serialize)]
struct Stop {
    id: String,
    modes: Vec<&'static str>,
}

#[derive(Serialize)]
struct Station {
    /// Hub code if the station has one, otherwise its first stop ID
    id: String,
    name: String,
    modes: Vec<&'static str>,
    stops: Vec<Stop>,
}

/// A stop point after merging duplicate listings across modes.
struct StopInfo {
    name: String,
    hub: Option<String>,
    modes: Vec<&'static str>,
}

fn mode_rank(mode: &str) -> usize {
    MODES
        .iter()
        .position(|(_, m)| *m == mode)
        .unwrap_or(usize::MAX)
}

/// Minimal union-find over stop indices.
struct DisjointSet(Vec<usize>);

impl DisjointSet {
    fn find(&mut self, i: usize) -> usize {
        if self.0[i] != i {
            let root = self.find(self.0[i]);
            self.0[i] = root;
        }
        self.0[i]
    }

    fn union(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        self.0[a.max(b)] = a.min(b);
    }
}

/// Group stops that share a TfL hub (e.g. Bank tube + DLR) or a normalised
/// name (e.g. both Hammersmith tube stations).
fn group_stops(stops: BTreeMap<String, StopInfo>) -> Vec<Station> {
    let entries: Vec<(String, StopInfo)> = stops.into_iter().collect();
    let mut set = DisjointSet((0..entries.len()).collect());
    let mut first_with_key: HashMap<String, usize> = HashMap::new();

    for (i, (_, info)) in entries.iter().enumerate() {
        let keys = [
            Some(format!("name:{}", info.name)),
            info.hub.as_ref().map(|h| format!("hub:{h}")),
        ];
        for key in keys.into_iter().flatten() {
            let first = *first_with_key.entry(key).or_insert(i);
            set.union(first, i);
        }
    }

    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..entries.len() {
        groups.entry(set.find(i)).or_default().push(i);
    }

    let mut stations: Vec<Station> = groups
        .into_values()
        .map(|members| {
            // Most common name wins, then the shortest
            let mut name_counts: HashMap<&str, usize> = HashMap::new();
            for &i in &members {
                *name_counts.entry(&entries[i].1.name).or_default() += 1;
            }
            let name = name_counts
                .into_iter()
                .min_by_key(|(name, count)| (std::cmp::Reverse(*count), name.len(), *name))
                .map(|(name, _)| name.to_string())
                .unwrap_or_default();

            let mut stops: Vec<Stop> = members
                .iter()
                .map(|&i| Stop {
                    id: entries[i].0.clone(),
                    modes: entries[i].1.modes.clone(),
                })
                .collect();
            stops.sort_by(|a, b| {
                (mode_rank(a.modes[0]), &a.id).cmp(&(mode_rank(b.modes[0]), &b.id))
            });

            let mut modes: Vec<&'static str> = stops.iter().flat_map(|s| s.modes.clone()).collect();
            modes.sort_by_key(|m| mode_rank(m));
            modes.dedup();

            let id = members
                .iter()
                .find_map(|&i| entries[i].1.hub.clone())
                .unwrap_or_else(|| stops[0].id.clone());

            Station {
                id,
                name,
                modes,
                stops,
            }
        })
        .collect();

    stations.sort_by(|a, b| a.name.cmp(&b.name));
    stations
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut stops: BTreeMap<String, StopInfo> = BTreeMap::new();

    for (mode_id, mode_name) in MODES {
        let url = format!("https://api.tfl.gov.uk/StopPoint/Mode/{mode_id}");
        let response: ModeResponse = client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let stations = response.stop_points.into_iter().filter(|p| {
            matches!(
                p.stop_type.as_str(),
                "NaptanMetroStation" | "NaptanRailStation"
            )
        });

        for point in stations {
            // Some stop IDs are listed under more than one mode
            let info = stops.entry(point.id).or_insert_with(|| StopInfo {
                name: normalize_station_name(&point.common_name),
                hub: point.hub_naptan_code,
                modes: Vec::new(),
            });
            if !info.modes.contains(&mode_name) {
                info.modes.push(mode_name);
            }
        }
    }

    let stop_count = stops.len();
    let stations = group_stops(stops);

    // Names that still look unnormalised may need a new rule in station_names
    for station in &stations {
        if station.name.ends_with("Station")
            || station.name.contains("Line)")
            || station.name.contains("Underground")
        {
            println!("warning: check name normalisation for {:?}", station.name);
        }
    }

    let output = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("generator is inside the workspace")
        .join("webapp")
        .join("stop_points.json");
    let mut json = serde_json::to_string_pretty(&stations)?;
    json.push('\n');
    std::fs::write(&output, json)?;

    println!(
        "Wrote {} stations ({} stop IDs) to {}",
        stations.len(),
        stop_count,
        output.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stop(id: &str, name: &str, hub: Option<&str>, mode: &'static str) -> (String, StopInfo) {
        (
            id.to_string(),
            StopInfo {
                name: name.to_string(),
                hub: hub.map(str::to_string),
                modes: vec![mode],
            },
        )
    }

    #[test]
    fn groups_by_hub_and_by_name() {
        let stations = group_stops(BTreeMap::from([
            stop("940GZZDLBNK", "Bank", Some("HUBBAN"), "DLR"),
            stop("940GZZLUBNK", "Bank", Some("HUBBAN"), "Underground"),
            stop("940GZZLUERB", "Edgware Road", None, "Underground"),
            stop("940GZZLUERC", "Edgware Road", None, "Underground"),
            stop("910GEUSTON", "Euston", Some("HUBEUS"), "Overground"),
            stop(
                "940GZZLUEUS",
                "Euston Square",
                Some("HUBEUS"),
                "Underground",
            ),
            stop("940GZZLUOXC", "Oxford Circus", None, "Underground"),
        ]));

        let summary: Vec<(&str, &str, usize)> = stations
            .iter()
            .map(|s| (s.id.as_str(), s.name.as_str(), s.stops.len()))
            .collect();
        assert_eq!(
            summary,
            [
                ("HUBBAN", "Bank", 2),
                ("940GZZLUERB", "Edgware Road", 2),
                ("HUBEUS", "Euston", 2),
                ("940GZZLUOXC", "Oxford Circus", 1),
            ]
        );
        // Stops and modes follow the display order of modes
        assert_eq!(stations[0].stops[0].id, "940GZZLUBNK");
        assert_eq!(stations[0].modes, ["Underground", "DLR"]);
    }
}
