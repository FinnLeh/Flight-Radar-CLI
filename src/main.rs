use std::error::Error;
use clap::Parser;
use tabled::settings::Style;
use std::{thread, time};
use warp::Filter;
use models::{Args, AirplanesLiveResponse, DefenseDisplay};
use std::sync::{Arc, Mutex};

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

    // Shared State:
    // Arc: Allows multiple owners access.
    // Mutex: Ensures that only one is writing at any time.
    let shared_anomalies = Arc::new(Mutex::new(Vec::<DefenseDisplay>::new()));

    // If KML is active in args, create the network link:
    if args.kml {
        println!("Starting KML Server at http://127.0.0.1:3030/kml ...");

        let server_data = shared_anomalies.clone();

        // Define Route:
        let kml_route = warp::path("kml")
            .map(move || {
                let planes = server_data.lock().unwrap();
                let kml_string = kml::generate_kml_string(&planes);

                warp::reply::with_header(
                    kml_string,
                    "Content-Type",
                    "application/vnd.google-earth.kml+xml"
                )
            });

        // Start server in the background (non-blocking)
        tokio::spawn(async move {
            warp::serve(kml_route).run(([127, 0, 0, 1], 3030)).await;
        });

        // Create Link File (pointing to localhost)
        kml::create_network_link("radar_link.kml")?;
    }

    // HTTP Request:
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.airplanes.live/v2/point/{}/{}/{}",
        lat, lon, args.radius
    );

    // Endless loop:
    loop {
        // empty screen (ANSI escape code):
        print!("\x1B[2J\x1B[1;1H");

        println!(" --- LIVE RADAR SCAN --- ");
        println!("Zeit: {:?}", chrono::Local::now().format("%H:%M:%S").to_string());
        println!("Sektor: {:.4}, {:.4} | Radius: {}nm", lat, lon, args.radius);

        // send request:
        match client.get(&url).send().await {
            Ok(resp) => {
                match resp.json::<AirplanesLiveResponse>().await {
                    Ok(data) => {
                        let aircraft_list = data.ac.unwrap_or_default();

                        // Filter Map
                        let anomalies: Vec<DefenseDisplay> = aircraft_list.iter()
                            .filter_map(|ac| {
                                // Check the plane:
                                match ac.check_interest(&args) {
                                    Some(reason) => Some(DefenseDisplay::new(ac, reason, &db)), // Hit! Return values plus Reason
                                    None => None,
                                }
                            })
                            .collect();

                        // Update KML
                        if args.kml {
                            {
                                let mut data = shared_anomalies.lock().unwrap();
                                *data = anomalies.clone();
                            }
                        }

                        if anomalies.is_empty() {
                            println!("Status: No targets.");
                        } else {
                            println!("ALERT: {} targets found!", anomalies.len());

                            // Show table
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

