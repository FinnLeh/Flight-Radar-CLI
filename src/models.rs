use std::ffi::CString;
use serde::Deserialize;
use serde_json::Value;
use clap::{Arg, Parser};
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

    /// Minimum height in meters (for example for Drone Finding)
    #[arg(long)]
    pub min_alt: Option<f64>,

    /// Maximum height in meters (for example, to find low flights)
    #[arg(long)]
    pub max_alt: Option<f64>,
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
    pub geo_altitude: Option<f64>,
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
        let geo_altitude = values[13].as_f64().or(values[7].as_f64());

        // Returning a StateVector if all values are there, or None otherwise
        Some(StateVector {
            icao24,
            callsign,
            origin_country,
            longitude,
            latitude,
            on_ground,
            velocity,
            geo_altitude,
        })
    }

    pub fn check_anomalies(&self, args: &Args) -> Option<String> {
        let speed = self.velocity.unwrap_or(0.0);
        let alt = self.geo_altitude.unwrap_or(0.0);
        let mut reasons = Vec::new();

        // Hard Filter: Ignore any flights outside the height boundaries:
        if let Some(min) = args.min_alt {
            if alt < min { return None; } // below the min height
        }
        // If max_alt is set, check it:
        if let Some(max) = args.max_alt {
            if alt > max { return None; } // above the max height
        }

        // Trigger 1: Speed.
        if speed > args.speed {
            reasons.push(format!("Speed ({:.0} > {})", speed, args.speed));
        }

        // Trigger 2: Origin Country
        // E.g., Russia:
        if self.origin_country == args.country {
            reasons.push("Country".to_string());
        }

        // Trigger 3: Geofence
        if let (Some(target_lat), Some(target_lon)) = (args.lat, args.lon) {
            // Check if the flight has coords for us to calculate with:
            if let (Some(plane_lat), Some(plane_lon)) = (self.latitude, self.longitude) {
                let distance = harversine_distance(target_lat, target_lon, plane_lat, plane_lon);

                if distance < args.radius {
                    reasons.push(format!("Geofence ({:.1}km)", distance));
                }
            }
        }

        if reasons.is_empty() {
            None // No anomaly
        } else {
            Some(reasons.join(", "))
        }
    }
}

#[derive(Tabled)]
pub struct AnomalyDisplay {
    icao: String,
    callsign: String,
    country: String,
    #[tabled(rename = "Velocity (m/s)")]
    velocity: f64,
    #[tabled(rename = "Alt (m)")]
    altitude: f64,
    #[tabled(rename = "Dist (km)")]
    distance: String, // String, so that it can display N/A if no target specified
    #[tabled(rename = "On Ground")]
    on_ground: bool,
    #[tabled(rename = "Reason")]
    reason: String,
}

impl AnomalyDisplay {
    pub fn new(s: &StateVector, args: &Args, reason: String) -> Self {
        let dist_str = if let (Some(lat), Some(lon), Some(p_lat), Some(p_lon))
            = (args.lat, args.lon, s.latitude, s.longitude) {
            let d = harversine_distance(lat, lon, p_lat, p_lon);
            format!("{:.1}", d)
        } else {
            "N/A".to_string()
        };

        Self {
            icao: s.icao24.clone(),
            callsign: s.callsign.clone(),
            country: s.origin_country.clone(),
            velocity: s.velocity.unwrap_or(0.0),
            altitude: s.geo_altitude.unwrap_or(0.0),
            distance: dist_str,
            on_ground: s.on_ground,
            reason,
        }
    }
}