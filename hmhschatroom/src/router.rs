use axum::{routing::get, Extension, Router};
use shuttle_runtime::SecretStore;
use sqlx::PgPool;

use crate::routes;
use tokio::sync::broadcast::{channel, Sender};
pub type RoomsStream = Sender<i32>;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub secrets: SecretStore,
}

pub fn init_router(db: PgPool, secrets: SecretStore) -> Router {
    let (tx, _rx) = channel::<i32>(10);
    let state = AppState { db, secrets };

    Router::new()
        .route("/", get(routes::home))
        .route("/styles.css", get(routes::styles))
        .route("/tailwind.css", get(routes::tailwind))
        .route("/rooms", get(routes::fetch_rooms).post(routes::create_room))
        .route(
            "/rooms/:id",
            get(routes::room)
                .delete(routes::delete_room)
                .post(routes::send_message),
        )
        .route("/rooms/:id/stream", get(routes::handle_stream))
        .route("/rooms/:id/messages", get(routes::messages))
        .route("/sign-in", get(routes::sign_in_page).post(routes::sign_in))
        .with_state(state)
        .layer(Extension(tx))
}
