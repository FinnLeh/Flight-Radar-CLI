use std::error::Error;
use clap::Parser;
use tabled::settings::Style;
use std::{thread, time};
use models::{Args, AirplanesLiveResponse, DefenseDisplay};

mod geo;
mod models;
mod db;
mod kml;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse arguments:
    let mut args = Args::parse();

    // load DB:
    println!("Loading Aircraft Database...");
    let db = db::load_database()?;
    println!("Loaded DB.");

    // Resolve Location:
    if let Some(loc) = &args.location {
        println!("Resolving location: '{}'...", loc);
        let (lat, lon) = geo::resolve_location(loc).await?;
        println!("--> Found coordinates: {:.4}, {:.4}", lat, lon);

        // put found values in args:
        args.lat = Some(lat);
        args.lon = Some(lon);
    }

    if args.lat.is_none() || args.lon.is_none() {
        eprintln!("Error: Please specify --location or --lat/--lon");
        return Ok(());
    }

    let lat = args.lat.unwrap();
    let lon = args.lon.unwrap();

    // If KML is active, create the Network Link
    if args.kml {
        println!("Creating Network Link...");
        kml::create_network_link("radar_link.kml")?;
        println!("DONE! Open 'radar_link.kml' in Google Earth now.");
        println!("System is starting Live-Scan in 3 Seconds...");
        thread::sleep(time::Duration::from_secs(3));
    }

    // HTTP Request:
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.airplanes.live/v2/point/{}/{}/{}",
        lat, lon, args.radius
    );

    // Endless Loop
    loop {
        // Empty Screen (ANSI Escape Code)
        print!("\x1B[2J\x1B[1;1H");

        println!(" --- LIVE RADAR SCAN --- ");
        println!("Time: {:?}", chrono::Local::now().format("%H:%M:%S").to_string());
        println!("Sector: {:.4}, {:.4} | Radius: {}nm", lat, lon, args.radius);

        // Send Request
        match client.get(&url).send().await {
            Ok(resp) => {
                match resp.json::<AirplanesLiveResponse>().await {
                    Ok(data) => {
                        let aircraft_list = data.ac.unwrap_or_default();

                        // Filter Anomalies
                        let anomalies: Vec<DefenseDisplay> = aircraft_list.iter()
                            .filter_map(|ac| {
                                match ac.check_interest(&args) {
                                    Some(reason) => Some(DefenseDisplay::new(ac, reason, &db)),
                                    None => None,
                                }
                            })
                            .collect();

                        if anomalies.is_empty() {
                            println!("Status: Green. No targets.");
                            // Write empty KML to make points in Google Earth disappear
                            if args.kml {
                                let _ = kml::save_kml("intelligence.kml", &Vec::new());
                            }
                        } else {
                            println!("ALERT: {} targets found!", anomalies.len());

                            // KML Update
                            if args.kml {
                                if let Err(e) = kml::save_kml("intelligence.kml", &anomalies) {
                                    eprintln!("KML Error: {}", e);
                                }
                            }

                            // Show Table:
                            let mut table = tabled::Table::new(anomalies);
                            table.with(Style::modern());
                            println!("{}", table);
                        }
                    },
                    Err(e) => eprintln!("JSON Error: {}", e),
                }
            },
            Err(e) => eprintln!("Connection Error: {}", e),
        }
        println!("\nNext Scan in 10 seconds...");
        thread::sleep(time::Duration::from_secs(10));
    }
}

