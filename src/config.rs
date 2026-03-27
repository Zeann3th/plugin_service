pub struct Config {
    pub service_host: String,
    pub service_port: u16,
    pub database_url: String,
    pub jwt_secret: String,
}

pub fn load_config() -> Result<Config, String> {
    dotenvy::dotenv().ok();

    let get_env = |key: &str| {
        std::env::var(key).map_err(|_| format!("Missing environment variable: {}", key))
    };

    Ok(Config {
        service_host: get_env("SERVICE_HOST")?,
        service_port: get_env("SERVICE_PORT")?
            .parse::<u16>()
            .map_err(|_| "SERVICE_PORT must be a number".to_string())?,
        database_url: get_env("DATABASE_URL")?,
        jwt_secret: get_env("JWT_SECRET")?,
    })
}
