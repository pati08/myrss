use std::sync::Arc;

use crate::{models::Message, routes};
use axum::{
    routing::{get, post},
    Extension, Router,
};
use groq_api_rust::AsyncGroqClient;
use tokio::sync::broadcast::{channel, Sender};
use tower_http::services::ServeDir;
pub type RoomsStream = Sender<Message>;

#[derive(Clone)]
pub struct AppState {
    pub groq_client: Arc<AsyncGroqClient>,
}

pub async fn init_router(groq_api_key: String) -> Router {
    let (tx, _rx) = channel::<Message>(10);

    let serve_assets = ServeDir::new("assets");
    let groq_client = AsyncGroqClient::new(groq_api_key, None).await;

    Router::new()
        .route("/", get(routes::home))
        .route("/feed", get(routes::feed))
        .route("/stream", get(routes::handle_stream))
        .route("/setname", post(routes::set_name))
        .route("/send", post(routes::send_message))
        .fallback_service(serve_assets)
        .layer(Extension(tx))
        .with_state(AppState {
            groq_client: Arc::new(groq_client),
        })
}
