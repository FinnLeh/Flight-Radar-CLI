use std::error::Error;
use clap::Parser;
use tabled::settings::Style;

mod geo;
mod models;

use models::{Args, AirplanesLiveResponse, DefenseDisplay};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse arguments:
    let args = Args::parse();

    // HTTP Request:
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.airplanes.live/v2/point/{}/{}/{}",
        args.lat, args.lon, args.radius
    );

    println!("Scanning airspace at {}, {} (Radius {} nm)...", args.lat, args.lon, args.radius);
    println!("Target: High Speed > {} kts, or HVT", args.speed);

    let resp = client.get(&url)
        .send()
        .await?
        .json::<AirplanesLiveResponse>()
        .await?;

    // If "ac" is none (no aircrafts), return empty vector
    let aircraft_list = resp.ac.unwrap_or_default();
    println!("Parsed: {} Aircrafts in the Sector.", aircraft_list.len());

    // Filtering the anomalies:
    let anomalies: Vec<DefenseDisplay> = aircraft_list.iter()
        .filter_map(|ac| {
            // Check the plane:
            match ac.check_interest(&args) {
                Some(reason) => Some(DefenseDisplay::new(ac, reason)), // Hit! Return values plus Reason
                None => None,
            }
        })
        .collect();

    if anomalies.is_empty() {
        println!("No relevant targets found.");
    } else {
        println!("{} High Value / Anomalies detected:", anomalies.len());
        let mut table = tabled::Table::new(anomalies);
        table.with(Style::modern());
        println!("{}", table);
    }

    Ok(())
}

