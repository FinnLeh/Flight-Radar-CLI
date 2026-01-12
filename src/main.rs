use std::error::Error;
use clap::Parser;
use tabled::settings::Style;

mod geo;
mod models;
mod db;
mod kml;

use models::{Args, AirplanesLiveResponse, DefenseDisplay};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse arguments:
    let mut args = Args::parse();

    // load DB:
    println!("Loading Aircraft Database...");
    let db = db::load_database()?;
    println!("Loaded DB.");

    // Resolve Location:
    if let Some(loc) = &args.location {
        println!("Resolving location: '{}'...", loc);
        let (lat, lon) = geo::resolve_location(loc).await?;
        println!("--> Found coordinates: {:.4}, {:.4}", lat, lon);

        // put found values in args:
        args.lat = Some(lat);
        args.lon = Some(lon);
    }

    if args.lat.is_none() || args.lon.is_none() {
        eprintln!("Error: Please specify --location or --lat/--lon");
        return Ok(());
    }

    let lat = args.lat.unwrap();
    let lon = args.lon.unwrap();

    // HTTP Request:
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.airplanes.live/v2/point/{}/{}/{}",
        lat, lon, args.radius
    );

    println!("Scanning Sector...");
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
                Some(reason) => Some(DefenseDisplay::new(ac, reason, &db)), // Hit! Return values plus Reason
                None => None,
            }
        })
        .collect();

    if anomalies.is_empty() {
        println!("No relevant targets found.");
    } else {
        println!("{} High Value / Anomalies detected:", anomalies.len());

        if args.kml && !anomalies.is_empty() {
            println!("Generating KML File...");
            let filename = "intelligence.kml";
            match kml::save_kml(filename, &anomalies) {
                Ok(_) => println!("Success! File '{}' created. Open it in Google Earth.", filename),
                Err(e) => eprintln!("Error while writing KML: {}", e),
            }
        }

        let mut table = tabled::Table::new(anomalies);
        table.with(Style::modern());
        println!("{}", table);
    }

    Ok(())
}

