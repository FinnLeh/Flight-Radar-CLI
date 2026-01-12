use std::error::Error;
use std::fmt::format;
use serde::Deserialize;
use reqwest::header::USER_AGENT;

#[derive(Deserialize, Debug)]
struct NominatimResponse {
    // Nominatim returns strings
    lat: String,
    lon: String,
}

/// Calculates the Distance between two coords in km.
/// Uses the Haversine Formula for spherical Geometry.
pub fn harversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;

    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();

    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_KM * c
}

/// Asks OpenStreetMaps for the Coords of a location
pub async fn resolve_location(query: &str) -> Result<(f64, f64), Box<dyn Error>> {
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