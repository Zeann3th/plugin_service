pub struct Config {
    pub service_host: String,
    pub service_port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_refresh_secret: String,
    pub s3_access_key_id: String,
    pub s3_secret_access_key: String,
    pub s3_endpoint: String,
    pub s3_region: String,
    pub s3_bucket: String,
    pub cors_allow_list: String,
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
        jwt_refresh_secret: get_env("JWT_REFRESH_SECRET")?,
        s3_access_key_id: get_env("S3_ACCESS_KEY_ID")?,
        s3_secret_access_key: get_env("S3_SECRET_ACCESS_KEY")?,
        s3_endpoint: get_env("S3_ENDPOINT")?,
        s3_region: get_env("S3_REGION")?,
        s3_bucket: get_env("S3_BUCKET")?,
        cors_allow_list: get_env("CORS_ALLOW_LIST")?,
    })
}
