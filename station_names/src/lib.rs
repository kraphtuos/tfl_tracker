//! Station name normalisation shared by the webapp and the station data generator.

/// Turn a TfL stop point or destination name into the name shown to users,
/// e.g. "London Euston Rail Station" -> "Euston".
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
