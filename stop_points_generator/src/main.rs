use reqwest;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Deserialize, Serialize, Debug, Clone)]
struct StopPoint {
    id: String,
    #[serde(rename = "commonName")]
    common_name: String,
    #[serde(rename = "stopType")]
    stop_type: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Fetch tube stations
    let response = reqwest::get("https://api.tfl.gov.uk/StopPoint/Mode/tube")
        .await?
        .json::<serde_json::Value>()
        .await?;

    let all: Vec<StopPoint> = serde_json::from_value(response["stopPoints"].clone())?;

    // Filter for metro stations only
    let mut tube_stations: Vec<StopPoint> = all
        .into_iter()
        .filter(|s| s.stop_type == "NaptanMetroStation")
        .collect();

    // Sort by name
    tube_stations.sort_by(|a, b| a.common_name.cmp(&b.common_name));

    // Save to file
    let json = serde_json::to_string_pretty(&tube_stations)?;
    fs::write(Path::new("stop_points.json"), json)?;

    println!(
        "Successfully created stop_points.json with {} stations",
        tube_stations.len()
    );
    Ok(())
}