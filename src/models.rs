use serde::{Deserialize, Serialize};
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

    /// Filter for Aircraft Type Code (e.g., "K35R")
    #[arg(short, long)]
    pub aircraft_type: Option<String>,

    /// Latitude of the target
    #[arg(long)]
    pub lat: f64,

    /// Longitude of the target
    #[arg(long)]
    pub lon: f64,

    /// Radius around the target in nautical miles
    #[arg(short, long, default_value_t = 250.0)]
    pub radius: f64,

    /// Minimum height in meters (for example for Drone Finding)
    #[arg(long)]
    pub min_alt: Option<f64>,

    /// Maximum height in meters (for example, to find low flights)
    #[arg(long)]
    pub max_alt: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct AirplanesLiveResponse {
    pub ac: Option<Vec<Aircraft>>, // Option, in case there are no planes available for some reason
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Aircraft {
    #[serde(rename = "hex")]
    pub icao: String,
    #[serde(rename = "flight")]
    pub callsign: Option<String>, // often empty for military planes
    #[serde(rename = "t")]
    pub type_code: Option<String>,
    #[serde(rename = "r")]
    pub registration: Option<String>,
    #[serde(rename = "gs")]
    pub ground_speed: Option<f64>, // ground speed in knots
    #[serde(rename = "alt_baro")]
    pub alt_baro: Option<f64>, // height in feet
    #[serde(rename = "alt_geom")]
    pub alt_geom: Option<f64>, // GPS height in feet
    #[serde(rename = "type")]
    pub source_type: String, // "adsb", "mlat" <-- This is the Ghost detector

    // may be missing:
    pub lat: Option<f64>,
    pub lon: Option<f64>,

    #[serde(rename = "mil")]
    pub is_military: Option<bool>, // Airplanes.live often flags military aircrafts
}

#[derive(Tabled)]
pub struct DefenseDisplay {
    icao: String,
    #[tabled(rename = "Type")]
    type_code: String,
    callsign: String,
    #[tabled(rename = "Speed (kt)")]
    speed: f64,
    #[tabled(rename = "Alt (ft)")]
    alt: f64,
    #[tabled(rename = "Source")]
    source: String, // MLAT or ADS-B
    #[tabled(rename = "Mil")]
    is_mil: bool, // intelligence flag
    #[tabled(rename = "Reason")]
    reason: String,
}

impl Aircraft {
    // Intelligence Logic:
    pub fn check_interest(&self, args: &Args) -> Option<String> {
        let mut reasons = Vec::new();
        let speed = self.ground_speed.unwrap_or(0.0);
        let alt = self.alt_baro.unwrap_or(0.0);

        // 1. Hard Filter:
        if let Some(max) = args.max_alt {
            if alt > max { return None; }
        }

        // 2. Intelligence Triggers:
        // A. Speed:
        if speed > args.speed {
            reasons.push(format!("High Speed ({:.0} kts)", speed));
        }

        // B. MLAT Detection (Ghost Tracking)
        if self.source_type == "mlat" {
            // for now, simply flag it as mlat source:
            reasons.push("MLAT as source".to_string());
        }

        // C. Must Watch Type / High Value Target
        if let Some(t) = &self.type_code {
            // default list:
            let high_value = vec!["K35R", "A332", "E3TF", "C17", "A400"];
            if high_value.contains(&t.as_str()) {
                reasons.push(format!("HVT: {}", t));
            }

            // if user searched explicit types:
            if let Some(target_type) = &args.aircraft_type {
                if t.contains(target_type) {
                    reasons.push("Target Type Match".to_string());
                }
            }
        }

        // D. Explicit military flag from API
        if self.is_military.unwrap_or(false) {
            reasons.push("MIL FLAG".to_string());
        }

        if reasons.is_empty() {
            None
        } else {
            Some(reasons.join(", "))
        }
    }
}

impl DefenseDisplay {
    pub fn new(a: &Aircraft, reason: String) -> Self {
        Self {
            icao: a.icao.clone(),
            type_code: a.type_code.clone().unwrap_or("???".to_string()),
            callsign: a.callsign.clone().unwrap_or("".to_string()),
            speed: a.ground_speed.unwrap_or(0.0),
            alt: a.alt_baro.unwrap_or(0.0),
            source: a.source_type.clone(),
            is_mil: a.is_military.unwrap_or(false),
            reason,
        }
    }
}