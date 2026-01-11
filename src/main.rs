use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct OpenSkyResponse {
    time: i64,
    states: Vec<Vec<Value>>,
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

    println!("{:?}", response);
}