use std::error::Error;
use clap::Parser;
use tabled::settings::Style;

mod geo;
mod models;

use models::{Args, AnomalyDisplay, StateVector, OpenSkyResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse arguments:
    let args = Args::parse();
    println!("Searchin for Anomalies: Speed > {} m/s, Country: '{}'", args.speed, args.country);

    // HTTP Request:
    let client = reqwest::Client::new();
    let url = "https://opensky-network.org/api/states/all";

    println!("Getting data from OpenSky...");

    // This now sends a request (.send()) and waits (.await).
    // The '?' at the end replaces the .unwrap():
    // If there is an error, return it right away. If success, continue.
    let resp = client.get(url)
        .send()
        .await?
        .json::<OpenSkyResponse>() // we tell it directly to try and parse it as OpenSkyResponse Struct
        .await?;

    // Creating an empty vector to store the flights data.
    let mut flights = Vec::new();

    // Iterate through the vector of data that is in response.
    // Try to create the StateVector struct for each flight from the values.
    // If that worked, push the flight into the flights vector.
    for raw_state in resp.states {
        if let Some(flight) = StateVector::from_values(&raw_state) {
            flights.push(flight);
        }
    }

    println!("Parsed: {} Flights.", flights.len());

    // Filtering the anomalies:
    let anomalies: Vec<(&StateVector, String)> = flights.iter()
        .filter_map(|f| {
            // Check the plane:
            match f.check_anomalies(&args) {
                Some(reason) => Some((f, reason)), // Hit! Return values plus Reason
                None => None,
            }
        })
        .collect();

    println!("Anomalies found: {}", anomalies.len());

    // Convert anomalies into rows of the display format:
    let display_rows: Vec<AnomalyDisplay> = anomalies.iter()
        .map(|(flight, reason)| AnomalyDisplay::new(flight, &args, reason.clone())) // *f dereferences the &&StateVector
        .collect();

    // Build table as a mutale and save it so we can change it later:
    let mut table = tabled::Table::new(display_rows);
    // Style the table (with modern style, gives round edges):
    table.with(Style::modern());



    println!("{}", table);

    Ok(())
}

