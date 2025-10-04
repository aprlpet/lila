use std::{fs, path::Path};

use crate::models::Config;

const DEFAULT_CONFIG: &str = r#"server_host = "127.0.0.1"
server_port = 3000
storage_path = "./data/objects"
database_url = "sqlite:./data/metadata.db"
auth_token = "owo"
rate_limit_per_second = 10
rate_limit_burst_size = 20
max_upload_size_mb = 100
"#;

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();

        let path = Path::new("config.toml");

        if !path.exists() {
            fs::write(path, DEFAULT_CONFIG)?;
            tracing::info!("Created default config.toml");
        }

        let config_str = fs::read_to_string(path)?;

        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }
}
