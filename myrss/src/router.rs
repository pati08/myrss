use crate::{models::Message, routes};
use axum::{
    routing::{get, post},
    Extension, Router,
};
use tokio::sync::broadcast::{channel, Sender};
use tower_http::services::ServeDir;
pub type RoomsStream = Sender<Message>;

pub fn init_router() -> Router {
    let (tx, _rx) = channel::<Message>(10);

    let serve_assets = ServeDir::new("assets");

    Router::new()
        .route("/", get(routes::home))
        .route("/feed", get(routes::feed))
        .route("/stream", get(routes::handle_stream))
        .route("/setname", post(routes::set_name))
        .route("/send", post(routes::send_message))
        .fallback_service(serve_assets)
        .layer(Extension(tx))
}
