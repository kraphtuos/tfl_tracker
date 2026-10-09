use serde::Deserialize;

/// A stop ID served by one or more modes (e.g. `910GLIVST` is both Elizabeth and Overground).
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    pub id: String,
    pub modes: Vec<String>,
}

/// A station as shown to the user, combining every stop ID that serves it.
/// Generated into `stop_points.json` by `stop_points_generator`.
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

    /// Whether `id` is this station's ID or one of its stop IDs.
    pub fn has_id(&self, id: &str) -> bool {
        self.id == id || self.stops.iter().any(|s| s.id == id)
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
}
