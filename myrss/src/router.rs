use std::sync::{Arc, Mutex};

use crate::{models::Message, routes};
use async_openai::config::OpenAIConfig;
use axum::{
    routing::{get, post},
    Extension, Router,
};
use tokio::sync::broadcast::{channel, Sender};
use tower_http::services::ServeDir;
pub type RoomsStream = Sender<Message>;

use super::create_system_messsage;

#[derive(Clone)]
pub struct AppState {
    pub ai_client: Arc<async_openai::Client<OpenAIConfig>>,
    pub ai_messages: Arc<Mutex<Vec<async_openai::types::ChatCompletionRequestMessage>>>,
    pub last_message: Arc<Mutex<Option<Message>>>,
}

pub async fn init_router(groq_api_key: String) -> Router {
    let (tx, _rx) = channel::<Message>(10);

    let serve_assets = ServeDir::new("assets");
    // let groq_client = AsyncGroqClient::new(groq_api_key, None).await;
    let groq_client = async_openai::Client::with_config(
        OpenAIConfig::new()
            .with_api_key(groq_api_key)
            .with_api_base("https://api.groq.com/openai/v1"),
    );

    Router::new()
        .route("/", get(routes::home))
        .route("/feed", get(routes::feed))
        .route("/stream", get(routes::handle_stream))
        .route("/setname", post(routes::set_name))
        .route("/send", post(routes::send_message))
        .fallback_service(serve_assets)
        .layer(Extension(tx))
        .with_state(AppState {
            ai_client: Arc::new(groq_client),
            ai_messages: Arc::new(Mutex::new(vec![create_system_messsage(
                include_str!("./aisysmsg.txt").to_string(),
            )])),
            last_message: Arc::new(Mutex::new(None)),
        })
}
