use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use chrono::{DateTime, Utc};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct Tariff {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    price: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_price: Option<u32>,
    duration: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Stream {
    user_id: Uuid,
    is_private: bool,
    settings: u32,
    shard_url: Url,
    public_tariff: Tariff,
    private_tariff: Tariff,
}

#[derive(Debug, Serialize, Deserialize)]
struct Gift {
    id: u32,
    price: u32,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DebugInfo {
    duration: String,
    at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    #[serde(rename = "type")]
    req_type: String,
    stream: Stream,
    gifts: Vec<Gift>,
    debug: DebugInfo,
}

pub fn run() {
    let file_path = Path::new("../request.json");
    let json_content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => {
            fs::read_to_string("request.json").expect("Failed to read request.json file")
        }
    };
    let request: Request = serde_json::from_str(&json_content)
        .expect("Failed to deserialize JSON");

    println!("=== Successfully Deserialized ===");
    let toml_string = toml::to_string_pretty(&request)
        .expect("Failed to serialize to TOML");

    println!("\n=== TOML Output ===\n");
    println!("{}", toml_string);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialization_serialization() {
        let data = r#"
        {
            "type": "success",
            "stream": {
                "user_id": "8d234120-0bda-49b2-b7e0-fbd3912f6cbf",
                "is_private": false,
                "settings": 45345,
                "shard_url": "https://n3.example.com/sapi",
                "public_tariff": {
                    "id": 1,
                    "price": 100,
                    "duration": "1h",
                    "description": "test public tariff"
                },
                "private_tariff": {
                    "client_price": 250,
                    "duration": "1m",
                    "description": "test private tariff"
                }
            },
            "gifts": [{
                "id": 1,
                "price": 2,
                "description": "Gift 1"
            }],
            "debug": {
                "duration": "234ms",
                "at": "2019-06-28T08:35:46+00:00"
            }
        }"#;

        let req: Request = serde_json::from_str(data).unwrap();

        assert_eq!(req.stream.settings, 45345);
        assert_eq!(req.stream.shard_url.host_str(), Some("n3.example.com"));

        let toml_output = toml::to_string(&req).unwrap();
        assert!(toml_output.contains("n3.example.com"));
    }
}