use serde::Deserialize;


#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub cache_url: Option<String>,
    pub cache_control_header: Option<String>,
    pub allowed_origins: Option<String>,
    pub disable_gzip: Option<bool>,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        dotenvy::dotenv().ok();
        config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }
}