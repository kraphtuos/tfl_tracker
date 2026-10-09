use serde::Deserialize;
use std::collections::HashMap;

/// A stop ID served by one or more modes (e.g. `910GLIVST` is both Elizabeth and Overground).
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    pub id: String,
    pub modes: Vec<String>,
}

/// A station as shown to the user, combining every stop ID that serves it.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub modes: Vec<String>,
    pub stops: Vec<Stop>,
}

impl Station {
    pub fn stop_ids(&self) -> Vec<String> {
        self.stops.iter().map(|s| s.id.clone()).collect()
    }
}

/// Entry of the raw `stop_points.json` produced by the generator.
#[derive(Deserialize)]
pub struct StopPoint {
    pub id: String,
    pub common_name: String,
    pub mode_name: String,
}

const MODE_ORDER: [&str; 4] = ["Underground", "Elizabeth", "DLR", "Overground"];

fn sort_modes(modes: &mut Vec<String>) {
    modes.sort_by_key(|m| MODE_ORDER.iter().position(|o| o == m).unwrap_or(usize::MAX));
    modes.dedup();
}

pub fn group_stations_by_name(stop_points: &[StopPoint]) -> Vec<Station> {
    let mut grouped: HashMap<String, Vec<Stop>> = HashMap::new();

    for point in stop_points {
        let stops = grouped
            .entry(normalize_station_name(&point.common_name))
            .or_default();
        // The same stop ID can be listed under several modes
        match stops.iter_mut().find(|s| s.id == point.id) {
            Some(stop) => stop.modes.push(point.mode_name.clone()),
            None => stops.push(Stop {
                id: point.id.clone(),
                modes: vec![point.mode_name.clone()],
            }),
        }
    }

    let mut result: Vec<Station> = grouped
        .into_iter()
        .map(|(name, mut stops)| {
            for stop in &mut stops {
                sort_modes(&mut stop.modes);
            }
            let mut modes: Vec<String> = stops.iter().flat_map(|s| s.modes.clone()).collect();
            sort_modes(&mut modes);
            Station {
                id: stops[0].id.clone(),
                name,
                modes,
                stops,
            }
        })
        .collect();

    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

pub fn normalize_station_name(name: &str) -> String {
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
        "Queens Park" => "Queen's Park".to_string(),

        _ => normalized,
    }
}

/// Lowercase, drop punctuation and spell out "&" so that e.g. "kings cross"
/// matches "King's Cross St. Pancras".
fn search_key(text: &str) -> String {
    let mut key = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_alphanumeric() {
            key.extend(c.to_lowercase());
        } else if c == '&' {
            key.push_str(" and ");
        } else if c.is_whitespace() || c == '-' || c == '/' {
            key.push(' ');
        }
    }
    key.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Stations matching `query`, best matches first: names starting with the
/// query, then names with a word starting with it, then any other match.
pub fn search_stations<'a>(stations: &'a [Station], query: &str) -> Vec<&'a Station> {
    let query = search_key(query);
    if query.is_empty() {
        return stations.iter().collect();
    }

    let mut matches: Vec<(u8, &Station)> = stations
        .iter()
        .filter_map(|station| {
            let name = search_key(&station.name);
            let rank = if name.starts_with(&query) {
                0
            } else if name.contains(&format!(" {query}")) {
                1
            } else if name.contains(&query) {
                2
            } else {
                return None;
            };
            Some((rank, station))
        })
        .collect();

    // Stable sort keeps the alphabetical order within each rank
    matches.sort_by_key(|(rank, _)| *rank);
    matches.into_iter().map(|(_, station)| station).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn station(name: &str) -> Station {
        Station {
            id: name.to_string(),
            name: name.to_string(),
            modes: vec![],
            stops: vec![],
        }
    }

    fn names(results: Vec<&Station>) -> Vec<&str> {
        results.into_iter().map(|s| s.name.as_str()).collect()
    }

    #[test]
    fn search_ignores_punctuation() {
        let stations = [
            station("King's Cross St. Pancras"),
            station("Elephant & Castle"),
        ];
        assert_eq!(
            names(search_stations(&stations, "kings cross st")),
            ["King's Cross St. Pancras"]
        );
        assert_eq!(
            names(search_stations(&stations, "elephant and")),
            ["Elephant & Castle"]
        );
        assert_eq!(
            names(search_stations(&stations, "elephant &")),
            ["Elephant & Castle"]
        );
    }

    #[test]
    fn search_ranks_prefix_matches_first() {
        let stations = [
            station("Arsenal"),
            station("Hyde Park Corner"),
            station("Park Royal"),
            station("Parsons Green"),
        ];
        assert_eq!(
            names(search_stations(&stations, "par")),
            ["Park Royal", "Parsons Green", "Hyde Park Corner"]
        );
        assert_eq!(names(search_stations(&stations, "sen")), ["Arsenal"]);
    }

    #[test]
    fn empty_query_returns_everything() {
        let stations = [station("Bank"), station("Oval")];
        assert_eq!(search_stations(&stations, "  ").len(), 2);
    }

    #[test]
    fn normalizes_suffixes_and_special_cases() {
        assert_eq!(
            normalize_station_name("Paddington (H&C Line)-Underground"),
            "Paddington"
        );
        assert_eq!(
            normalize_station_name("London Euston Rail Station"),
            "Euston"
        );
        assert_eq!(
            normalize_station_name("Queens Park (London) Rail Station"),
            "Queen's Park"
        );
        assert_eq!(
            normalize_station_name("Hammersmith (Dist&Picc Line) Underground Station"),
            "Hammersmith"
        );
    }

    #[test]
    fn groups_stops_by_name_and_merges_duplicate_ids() {
        let points = [
            ("910GSTFD", "Stratford (London) Rail Station", "Overground"),
            (
                "940GZZLUSTD",
                "Stratford Underground Station",
                "Underground",
            ),
            ("910GSTFD", "Stratford (London) Rail Station", "Elizabeth"),
        ]
        .map(|(id, name, mode)| StopPoint {
            id: id.to_string(),
            common_name: name.to_string(),
            mode_name: mode.to_string(),
        });
        let stations = group_stations_by_name(&points);
        assert_eq!(stations.len(), 1);
        assert_eq!(stations[0].stops.len(), 2);
        assert_eq!(
            stations[0].modes,
            ["Underground", "Elizabeth", "Overground"]
        );
    }
}
