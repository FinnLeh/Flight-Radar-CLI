use serde::Deserialize;
use serde_json::Value;
use clap::Parser;
use tabled::Tabled;

use crate::geo::harversine_distance;

/// A simple CLI tool to scan OpenSky Data for Anomalies.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Speed Limit in m/s
    #[arg(short, long, default_value_t = 300.0)]
    pub speed: f64,

    /// Country that you want to search for
    #[arg(short, long, default_value = "Russian Federation")]
    pub country: String,

    /// Latitude of the target
    #[arg(long)]
    pub lat: Option<f64>,

    /// Longitude of the target
    #[arg(long)]
    pub lon: Option<f64>,

    /// Radius around the target in km
    #[arg(short, long, default_value_t = 100.0)]
    pub radius: f64,
}

#[derive(Debug, Deserialize)]
pub struct OpenSkyResponse {
    pub time: i64,
    pub states: Vec<Vec<Value>>,
}

#[derive(Debug)]
pub struct StateVector {
    pub icao24: String,
    pub callsign: String,
    pub origin_country: String,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub on_ground: bool,
    pub velocity: Option<f64>,
}

impl StateVector {
    pub fn from_values(values: &Vec<Value>) -> Option<Self> {
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

    pub fn is_anomaly(&self, args: &Args) -> bool {
        // Criteria 1: Speed.
        // Unwrap velocity:
        let speed = self.velocity.unwrap_or(0.0);
        if speed > args.speed {
            return true;
        }

        // Criteria 2: Origin
        // E.g., Russia:
        if self.origin_country == args.country {
            return true;
        }

        // Criteria 3: Geofence Check
        if let (Some(target_lat), Some(target_lon)) = (args.lat, args.lon) {
            // Check if the flight has coords for us to calculate with:
            if let (Some(plane_lat), Some(plane_lon)) = (self.latitude, self.longitude) {
                let distance = harversine_distance(target_lat, target_lon, plane_lat, plane_lon);

                if distance < args.radius {
                    return true;
                }
            }
        }

        false
    }
}

#[derive(Tabled)]
pub struct AnomalyDisplay {
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