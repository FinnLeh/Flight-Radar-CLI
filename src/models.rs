use std::iter::Copied;
use serde::{Deserialize, Serialize, Deserializer};
use serde_json::Value;
use clap::Parser;
use tabled::Tabled;
use crate::db::AircraftDB;


/// A simple CLI tool to scan OpenSky Data for Anomalies.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(group(
    clap::ArgGroup::new("coords")
        .required(true)
        .args(&["location", "lat"])
))]
pub struct Args {
    /// Speed Limit in m/s
    #[arg(short, long, default_value_t = 300.0)]
    pub speed: f64,

    /// Filter for Aircraft Type Code (e.g., "K35R")
    #[arg(short, long)]
    pub aircraft_type: Option<String>,

    /// Name of the target location (e.g., "London")
    #[arg(short = 'L', long)]
    pub location: Option<String>,

    /// Latitude of the target (will be ignored if location is set)
    #[arg(long)]
    pub lat: Option<f64>,

    /// Longitude of the target (will be ignored if location is set)
    #[arg(long)]
    pub lon: Option<f64>,

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

// Helper function for dirty data:
fn parse_altitude<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    // Read the field as a generic JSON Value
    let v = Value::deserialize(deserializer)?;

    match v {
        Value::Number(n) => Ok(n.as_f64()), // if it's a number, take that
        Value::String(s) if s == "ground" => Ok(Some(0.0)), // if it says "ground", turn it into 0.0
        _ => Ok(None), // everything else turns into None
    }
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
    #[serde(rename = "alt_baro", deserialize_with = "parse_altitude", default)]
    pub alt_baro: Option<f64>, // height in feet
    #[serde(rename = "alt_geom", deserialize_with = "parse_altitude", default)]
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
    #[tabled(rename = "Operator")]
    operator: String,
    callsign: String,
    #[tabled(rename = "Speed (kt)")]
    speed: f64,
    #[tabled(rename = "Alt (ft)")]
    alt: f64,
    #[tabled(rename = "Source")]
    source: String, // MLAT or ADS-B
    #[tabled(rename = "Reason")]
    reason: String,
}

impl Aircraft {
    // Intelligence Logic:
    pub fn check_interest(&self, args: &Args) -> Option<String> {
        let mut reasons = Vec::new();
        let speed = self.ground_speed.unwrap_or(0.0);
        let alt = self.alt_baro.unwrap_or(0.0);
        let type_code = self.type_code.clone().unwrap_or_default();

        // 1. Hard Filter:
        if let Some(max) = args.max_alt {
            if alt > max { return None; }
        }

        // 2. Intelligence Triggers:
        // A. Speed and Altitude:
        if (alt < 25000.0 && speed > args.speed) || speed > 550.0 {
            reasons.push(format!("Speed ({:.0} kts @ {:.0} ft)", speed, alt));
        }

        // B. MLAT Detection (Ghost Tracking)
        // List of boring small planes we want to ignore:
        let boring_types = vec!["C172", "C152", "P28A", "DA40", "R44", "G115"];

        if self.source_type == "mlat" {
            if !boring_types.contains(&type_code.as_str()) {
                // for now, simply flag it as mlat source:
                reasons.push("MLAT as source".to_string());
            }
        }

        // C. High Value Target (HVT)
        if let Some(t) = &self.type_code {
            // default list:
            let high_value = vec![
                "K35R", "K46", "A332", "E3TF", "C17", "A400", // Tanker/Transport
                "B52", "B1", "B2", // Bomber
                "EUFI", "F35", "F16", "F18", "TORN" // Fighter
            ];
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
    pub fn new(a: &Aircraft, reason: String, db: &AircraftDB) -> Self {
        // Operator Lookup:
        let operator = if let Some(info) = db.get(&a.icao) {
            info.operator.clone().unwrap_or("Unknown".to_string())
        } else {
            "Unknown".to_string()
        };

        Self {
            icao: a.icao.clone(),
            type_code: a.type_code.clone().unwrap_or("???".to_string()),
            operator,
            callsign: a.callsign.clone().unwrap_or("".to_string()),
            speed: a.ground_speed.unwrap_or(0.0),
            alt: a.alt_baro.unwrap_or(0.0),
            source: a.source_type.clone(),
            reason,
        }
    }
}