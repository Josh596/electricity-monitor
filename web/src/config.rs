use std::env;

const DEFAULT_DATABASE_URL: &str = "sqlite:data.db";
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3000";
const DEFAULT_OUTAGE_THRESHOLD_SECS: i64 = 90;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_address: String,
    pub outage_threshold: chrono::TimeDelta,
    pub api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let database_url = env::var("DATABASE_URL").unwrap_or(DEFAULT_DATABASE_URL.to_string());
        let bind_address = env::var("BIND_ADDRESS").unwrap_or(DEFAULT_BIND_ADDR.to_string());
        let api_key = env::var("API_KEY").expect("API_KEY must be set in the environment");
        let threshold_secs: i64 = env::var("OUTAGE_THRESHOLD_SECS")
            .ok()
            .map(|v| {
                v.parse()
                    .expect("OUTAGE_THRESHOLD_SECS must be a whole number of seconds")
            })
            .unwrap_or(DEFAULT_OUTAGE_THRESHOLD_SECS);

        let outage_threshold = chrono::Duration::seconds(threshold_secs);

        Ok(Config {
            database_url,
            bind_address,
            outage_threshold,
            api_key,
        })
    }
}
