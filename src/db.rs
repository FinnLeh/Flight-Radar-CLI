use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use serde::Deserialize;

// this struct represents a single line in the CSV file
#[derive(Debug, Deserialize, Clone)]
pub struct AircraftInfo {
    #[serde(rename = "icao24")] // CSV Header name
    pub icao: String,

    // #[serde(rename = "manufacturername")]
    // pub manufacturer: Option<String>,

    // #[serde(rename = "model")]
    // pub model: Option<String>,

    #[serde(rename = "operator")]
    pub operator: Option<String>,
}

pub type AircraftDB = HashMap<String, AircraftInfo>;

pub fn load_database() -> Result<AircraftDB, Box<dyn Error>> {
    let file_path = "aircraft_db.csv";

    if File::open(file_path).is_err() {
        println!("WARNING: 'aircraft_db.csv' not found. No aircraft information available.");
        return Ok(HashMap::new());
    }

    let file = File::open(file_path)?;
    let mut rdr = csv::Reader::from_reader(file);
    let mut db = HashMap::new();

    // We iterate over every row.
    for result in rdr.deserialize() {
        // ignoring broken lines (happens for csv sometimes)
        if let Ok(record) = result {
            let info: AircraftInfo = record;
            // use ICAO Code as key for fast finding:
            db.insert(info.icao.clone(), info);
        }
    }

    Ok(db)
}