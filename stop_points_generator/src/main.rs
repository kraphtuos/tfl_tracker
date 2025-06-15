use reqwest;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize, Debug, Clone)]
struct StopPoint {
    id: String,
    common_name: String,
    stop_type: String,
    mode_name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Fetch stations for multiple modes
    let modes = ["tube", "dlr", "elizabeth-line", "overground"];
    let mut all_stations = Vec::new();

    for mode in modes {
        let url = format!("https://api.tfl.gov.uk/StopPoint/Mode/{}", mode);
        let response = reqwest::get(&url)
            .await?
            .json::<serde_json::Value>()
            .await?;

        // Convert API response into our simplified format
        let display_mode = match mode {
            "tube" => "Underground",
            "dlr" => "DLR",
            "elizabeth-line" => "Elizabeth line",
            "overground" => "Overground",
            _ => mode,
        };
        let stations: Vec<StopPoint> = response["stopPoints"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|point| StopPoint {
                id: point["id"].as_str().unwrap_or("").to_string(),
                common_name: point["commonName"].as_str().unwrap_or("").to_string(),
                stop_type: point["stopType"].as_str().unwrap_or("").to_string(),
                mode_name: display_mode.to_string(),
            })
            .collect();

        all_stations.extend(stations);
    }

    // Filter for relevant station types and clean names
    let mut stations: Vec<StopPoint> = all_stations
        .into_iter()
        .filter(|s| {
            matches!(
                s.stop_type.as_str(),
                "NaptanMetroStation" | "NaptanRailStation"
            )
        })
        .map(|mut s| {
            // Remove various suffix patterns
            let suffixes = [
                " Underground Station",
                " DLR Station",
                " Rail Station",
                " Station",
            ];
            for suffix in suffixes {
                if let Some(stripped) = s.common_name.strip_suffix(suffix) {
                    s.common_name = stripped.to_string();
                    break;
                }
            }
            s
        })
        .collect();

    // Sort by name
    stations.sort_by(|a, b| a.common_name.cmp(&b.common_name));

    // Save to file
    let json = serde_json::to_string_pretty(&stations)?;
    fs::write(Path::new("./webapp/stop_points.json"), &json)?;

    println!(
        "Successfully created stop_points.json with {} stations",
        stations.len()
    );
    Ok(())
}
