use crate::config::Config;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use std::sync::Arc;
use aws_sdk_s3::Client as S3Client;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub struct AppState {
    pub config: Config,
    pub db_pool: DbPool,
    pub s3_client: S3Client,
}

pub type SharedState = Arc<AppState>;
