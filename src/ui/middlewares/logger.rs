use axum::extract::MatchedPath;
use axum::{body::Body, http::Request, response::Response};
use std::time::Duration;
use tower_http::trace::TraceLayer;
use tracing::{info_span, Span};

pub fn logger_layer<B>() -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    impl FnMut(&Request<Body>) -> Span + Clone,
    impl FnMut(&Request<Body>, &Span) + Clone,
    impl FnMut(&Response<B>, Duration, &Span) + Clone,
> {
    TraceLayer::new_for_http()
        .make_span_with(|request: &Request<Body>| {
            let matched_path = request
                .extensions()
                .get::<MatchedPath>()
                .map(MatchedPath::as_str);

            info_span!(
                "http_request",
                method = %request.method(),
                path = %request.uri().path(),
                query = ?request.uri().query(),
                matched_path = %matched_path.unwrap_or("<unknown>"),
                status = tracing::field::Empty,
                latency = tracing::field::Empty,
            )
        })
        .on_request(|_request: &Request<Body>, _span: &Span| {
            // Noisy if we log on every request start, but good for debugging
        })
        .on_response(|response: &Response<B>, latency: Duration, span: &Span| {
            let status = response.status().as_u16();
            span.record("status", &status);
            span.record("latency", &format!("{:?}", latency));

            // Log completion with all relevant info
            tracing::info!(
                "completed"
            );
        })
}
