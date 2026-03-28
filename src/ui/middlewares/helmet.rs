use axum_helmet::{Helmet, HelmetLayer, XContentTypeOptions, XFrameOptions, XXSSProtection};

pub fn helmet_layer() -> HelmetLayer {
    Helmet::new()
        .add(XContentTypeOptions::nosniff())
        .add(XFrameOptions::same_origin())
        .add(XXSSProtection::on().mode_block())
        .into_layer()
        .unwrap()
}
