use std::error::Error;
use serde::Deserialize;
use reqwest::header::USER_AGENT;

#[derive(Deserialize, Debug)]
struct NominatimResponse {
    // Nominatim returns strings
    lat: String,
    lon: String,
}

 /*
/// Calculates the Distance between two coords in km.
/// Uses the Haversine Formula for spherical Geometry.
pub fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;

    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();

    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_KM * c
}
 */


// TODO: Make the list extendable by creating a locations.toml instead of the way it currently works. Then users can add their own locations that they want to locally save.
// Static Database to reduce Nominatim API usage:
fn get_static_coords(query: &str) -> Option<(f64, f64)> {
    // normalize input (everything lowercase)
    match query.to_lowercase().as_str() {
        "london" => Some((51.5074, -0.1278)),
        "mildenhall" => Some((52.3467, 0.4795)), // RAF Mildenhall
        "lakenheath" => Some((52.4093, 0.5606)), // RAF Lakenheath

        // Deutschland
        "berlin" => Some((52.5200, 13.4050)),
        "ramstein" | "ramstein air base" => Some((49.4365, 7.6003)),
        "spangdahlem" => Some((49.9745, 6.6923)),
        "geilenkirchen" => Some((50.9602, 6.0469)), // AWACS Basis

        // USA
        "washington" | "dc" => Some((38.9072, -77.0369)),
        "groom lake" | "area 51" => Some((37.2343, -115.8067)),
        "edwards afb" => Some((34.9056, -117.8837)),
        "norfolk" => Some((36.8508, -76.2859)), // Naval Station

        // Hotspots
        "kyiv" | "kiew" => Some((50.4501, 30.5234)),
        "tel aviv" => Some((32.0853, 34.7818)),
        "taipei" => Some((25.0330, 121.5654)),
        "kaliningrad" => Some((54.7104, 20.4522)),

        _ => None, // Nicht gefunden
    }
}

/// Asks OpenStreetMaps for the Coords of a location
pub async fn resolve_location(query: &str) -> Result<(f64, f64), Box<dyn Error>> {
    // Look into internal static database for locations first:
    if let Some(coords) = get_static_coords(query) {
        println!("(Offline-Cache used for '{}')", query);
        return Ok(coords);
    }

    let client = reqwest::Client::new();

    // URL for Nominatim Search
    let url = format!(
        "https://nominatim.openstreetmap.org/search?q={}&format=json&limit=1",
        query
    );

    // Send request (IMPORTANT: Include User-Agent Header!)
    let resp = client.get(&url)
        .header(USER_AGENT, "FlightRadarCLI/1.0 (finnleh5@gmail.com)")
        .send()
        .await?
        .json::<Vec<NominatimResponse>>() // expecting a list
        .await?;

    if let Some(place) = resp.first() {
        // Parse string in f64
        let lat = place.lat.parse::<f64>()?;
        let lon = place.lon.parse::<f64>()?;
        Ok((lat, lon))
    } else {
        Err(format!("Location '{}' could not be found.", query).into())
    }
}