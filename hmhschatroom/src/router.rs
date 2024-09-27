use axum::{routing::get, Extension, Router};
use sqlx::PgPool;

use crate::routes;
use tokio::sync::broadcast::{channel, Sender};
pub type RoomsStream = Sender<()>;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

pub fn init_router(db: PgPool) -> Router {
    let (tx, _rx) = channel::<()>(10);
    let state = AppState { db };

    Router::new()
        .route("/", get(routes::home))
        .route("/styles.css", get(routes::styles))
        .route("/rooms", get(routes::fetch_rooms).post(routes::create_room))
        .route("/rooms/:id", get(routes::room).delete(routes::delete_room))
        .route("/rooms/:id/stream", get(routes::handle_stream))
        .with_state(state)
        .layer(Extension(tx))
}
