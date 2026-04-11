use axum::{Router, routing::get};
use tracing_subscriber::EnvFilter;

async fn health() -> &'static str {
    "ok"
}

pub fn app() -> Router {
    Router::new().route("/health", get(health))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind");

    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app()).await.expect("server error");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn health_returns_ok() {
        let server = TestServer::new(app());
        let response = server.get("/health").await;
        response.assert_status_ok();
        response.assert_text("ok");
    }
}
