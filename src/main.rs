use serde::Deserialize;
use serde_json::Value;

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
}

fn main() {
    let raw_data = r#"
    {
        "time": 1683100000,
        "states": [
            ["4b1814", "HB-JWC", "Switzerland", 1620000000, 1620000000, 7.5, 47.5, 10000.5, false, 200.0]
        ]
    }
    "#;

    let response: OpenSkyResponse = serde_json::from_str(raw_data).unwrap();

    let mut flights = Vec::new();

    for raw_state in response.states {
        if let Some(flight) = StateVector::from_values(&raw_state) {
            flights.push(flight);
        }
    }

    println!("{:?}", flights);
}