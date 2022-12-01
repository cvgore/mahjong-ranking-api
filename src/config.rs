use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub database_max_conn: u32,
    pub database_conn_timeout: u64,
    pub bind_interface: String,
    pub firebase_project_id: String,
    pub database_pragma_cache_size: u32
}

pub fn init_config() -> Config {
    dotenvy::dotenv().ok();

    envy::from_env::<Config>().expect("could not load .env file")
}

