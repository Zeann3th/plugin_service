use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::config::{Builder, Credentials, Region};
use aws_sdk_s3::Client;
use crate::config::Config;

pub async fn connect(config: &Config) -> Client {
    let credentials = Credentials::new(
        &config.s3_access_key_id,
        &config.s3_secret_access_key,
        None,
        None,
        "Static",
    );

    let region_provider = RegionProviderChain::first_try(Region::new(config.s3_region.clone()));
    let shared_config = aws_config::from_env()
        .region(region_provider)
        .credentials_provider(credentials)
        .load()
        .await;

    let s3_config_builder = Builder::from(&shared_config)
        .endpoint_url(&config.s3_endpoint)
        .force_path_style(true);

    Client::from_conf(s3_config_builder.build())
}

pub async fn ensure_bucket_exists(client: &Client, bucket: &str) {
    let resp = client.list_buckets().send().await;

    match resp {
        Ok(output) => {
            let buckets = output.buckets();
            let exists = buckets.iter().any(|b| b.name() == Some(bucket));

            if !exists {
                tracing::info!("Creating bucket: {}", bucket);
                client.create_bucket().bucket(bucket).send().await.expect("Failed to create bucket");
            }
        }
        Err(e) => {
            tracing::error!("Failed to list buckets: {}", e);
        }
    }
}
