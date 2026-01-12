use std::error::Error;
use std::fmt::format;
use std::fs::File;
use std::io::Write;
use crate::models::DefenseDisplay;

fn get_header() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
    <kml xmlns="http://www.opengis.net/kml/2.2">
    <Document>
        <name>Flight Radar Intelligence</name>
        <Style id="style_mil">
            <IconStyle>
                <color>ff0000ff</color> <scale>1.2</scale>
                <Icon><href>http://maps.google.com/mapfiles/kml/shapes/airports.png</href></Icon>
            </IconStyle>
        </Style>
        <Style id="style_warn">
            <IconStyle>
                <color>ff00ffff</color> <scale>1.0</scale>
                <Icon><href>http://maps.google.com/mapfiles/kml/shapes/caution.png</href></Icon>
            </IconStyle>
        </Style>
        <Style id="style_norm">
            <IconStyle>
                <color>ffffffff</color> <scale>0.8</scale>
                <Icon><href>http://maps.google.com/mapfiles/kml/shapes/airports.png</href></Icon>
            </IconStyle>
        </Style>
    "#
}

pub fn save_kml(filename: &str, anomalies: &Vec<DefenseDisplay>) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(filename)?;
    file.write_all(get_header().as_bytes())?;

    for plane in anomalies {
        // Logic for color/style
        let style = if plane.reason.contains("HVT") || plane.reason.contains("MIL") || plane.reason.contains("Country") {
            "#style_mil" // red
        } else if plane.reason.contains("NAV") {
            "#style_warn" // yellow
        } else {
            "#style_norm" // white
        };

        let description = format!(
            "<b>Operator:</b> {}<br/><b>Type:</b> {}<br/><b>Callsign:</b> {}<br/><b>Speed:</b> {:.0} kts<br/><b>Alt:</b> {:.0} ft<br/><b>Reason:</b> {}",
            plane.operator, plane.type_code, plane.callsign, plane.speed, plane.alt, plane.reason
        );

        // Create KML Placemark
        let kml_placemark = format!(
            r#"
            <Placemark>
                <name>{}</name>
                <description><![CDATA[{}]]></description>
                <styleUrl>{}</styleUrl>
                <Point>
                    <coordinates>{},{},{}</coordinates>
                </Point>
            </Placemark>"#,
            plane.icao, description, style, plane.lon, plane.lat, plane.alt * 0.3048 // Alt in Metern für KML (optional)
        );

        file.write_all(kml_placemark.as_bytes())?;
    }

    file.write_all(b"\n</Document>\n</kml>")?;
    Ok(())
}

/// Creates the Link File, that tells Google Earth to load intelligence.kml anew regularly
pub fn create_network_link(filename: &str) -> Result<(), Box<dyn Error>> {
    let content = r#"<?xml version="1.0" encoding="UTF-8"?>
    <kml xmlns="http://www.opengis.net/kml/2.2">
        <NetworkLink>
            <name>Flight Radar Live Feed</name>
            <open>1</open>
            <Link>
                <href>intelligence.kml</href>
                <refreshMode>onInterval</refreshMode>
                <refreshInterval>5</refreshInterval> </Link>
        </NetworkLink>
    </kml>
    "#;
    let mut file = File::create(filename)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}