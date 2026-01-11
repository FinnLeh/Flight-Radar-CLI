use serde::Deserialize;
use serde_json::Value;
use std::error::Error;
use clap::Parser;
use tabled::{Tabled, settings::Style};

#[derive(Tabled)]
struct AnomalyDisplay {
    icao: String,
    callsign: String,
    country: String,
    #[tabled(rename = "Velocity (m/s)")]
    velocity: f64,
    #[tabled(rename = "On Ground")]
    on_ground: bool,
}

impl From<&StateVector> for AnomalyDisplay {
    fn from(s: &StateVector) -> Self {
        Self {
            icao: s.icao24.clone(),
            callsign: s.callsign.clone(),
            country: s.origin_country.clone(),
            velocity: s.velocity.unwrap_or(0.0),
            on_ground: s.on_ground,
        }
    }
}

/// A simple CLI tool to scan OpenSky Data for Anomalies.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Speed Limit in m/s
    #[arg(short, long, default_value_t = 300.0)]
    speed: f64,

    /// Country that you want to search for
    #[arg(short, long, default_value = "Russian Federation")]
    country: String,
}

#[derive(Debug, Deserialize)]
struct OpenSkyResponse {
    time: i64,
    states: Vec<Vec<Value>>,
}

#[derive(Debug)]
struct StateVector {
    icao24: String,
    callsign: String,
    origin_country: String,
    longitude: Option<f64>,
    latitude: Option<f64>,
    on_ground: bool,
    velocity: Option<f64>,
}

impl StateVector {
    fn from_values(values: &Vec<Value>) -> Option<Self> {
        // If there is less than 10 values, the array is broken and not usable
        if values.len() < 10 {
            return None;
        }

        // Map based on indices.
        // .as_str() returns Option<&str>.
        // .to_string() makes it a real String (Deep Copy/Heap allocation).
        // unwrap_or() takes a specified String, if it was Null.
        let icao24 = values[0].as_str().unwrap_or("").to_string();
        let callsign = values[1].as_str().unwrap_or("").to_string();
        let origin_country = values[2].as_str().unwrap_or("").to_string();

        // Numbers: .as_f64() returns Option<f64> (i.e., either the value or None if Null).
        let longitude = values[5].as_f64();
        let latitude = values[6].as_f64();
        let on_ground = values[8].as_bool().unwrap_or(false);
        let velocity = values[9].as_f64();

        // Returning a StateVector if all values are there, or None otherwise
        Some(StateVector {
            icao24,
            callsign,
            origin_country,
            longitude,
            latitude,
            on_ground,
            velocity,
        })
    }

    fn is_anomaly(&self, threshold_speed: f64, target_country: &str) -> bool {
        // Criteria 1: Speed.
        // Unwrap velocity:
        let speed = self.velocity.unwrap_or(0.0);
        if speed > threshold_speed {
            return true;
        }

        // Criteria 2: Origin
        // E.g., Russia:
        if self.origin_country == target_country {
            return true;
        }

        false
    }
}

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

    // Return the first 5 flights to check whether it works.
    // .iter().take(5) is like python slicing [:5]
    for flight in flights.iter().take(5) {
        println!("{:?}", flight);
    }

    // Filtering the anomalies:
    let anomalies: Vec<&StateVector> = flights.iter()
        .filter(|f| f.is_anomaly(args.speed, &args.country)) // lambda function (Closure)
        .collect();

    // Convert anomalies into rows of the display format:
    let display_rows: Vec<AnomalyDisplay> = anomalies.iter()
        .map(|f| AnomalyDisplay::from(*f)) // *f dereferences the &&StateVector
        .collect();

    // Build table as a mutale and save it so we can change it later:
    let mut table = tabled::Table::new(display_rows);
    // Style the table (with modern style, gives round edges):
    table.with(Style::modern());

    println!("Davon Anomalien gefunden: {}", anomalies.len());

    println!("{}", table);

    Ok(())
}
