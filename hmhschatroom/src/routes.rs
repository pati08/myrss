use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Response, Sse},
    Extension, Form,
};
use sqlx::types::chrono::{DateTime, Local};
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt as _};

use crate::models::{Message, MessageNew, Room, RoomNew};
use crate::{errors::ApiError, router::AppState, router::RoomsStream, templates};

pub async fn home() -> impl IntoResponse {
    templates::HelloTemplate
}

pub async fn fetch_rooms(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    println!("Getting rooms");
    let rooms = sqlx::query_as::<_, Room>("SELECT * FROM rooms")
        .fetch_all(&state.db)
        .await?;

    Ok(templates::Records { rooms })
}

pub async fn styles() -> Result<impl IntoResponse, ApiError> {
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/css")
        .body(include_str!("../templates/styles.css").to_owned())?;

    Ok(response)
}

pub async fn create_room(
    State(state): State<AppState>,
    Extension(tx): Extension<RoomsStream>,
    Form(form): Form<RoomNew>,
) -> impl IntoResponse {
    println!("Creating room");
    let room = sqlx::query_as::<_, Room>(
        "INSERT INTO rooms (name, description) VALUES ($1, $2) RETURNING id, name, description",
    )
    .bind(form.name)
    .bind(form.description)
    .fetch_one(&state.db)
    .await
    .unwrap();

    if tx.send(()).is_err() {
        eprintln!(
            "Record with ID {} was created but nobody's listening to the stream!",
            room.id
        );
    }

    templates::RoomNewTemplate { room }
}

pub async fn delete_room(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Extension(tx): Extension<RoomsStream>,
) -> Result<impl IntoResponse, ApiError> {
    sqlx::query("DELETE FROM rooms WHERE ID = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if tx.send(()).is_err() {
        eprintln!(
            "Record with ID {} was deleted but nobody's listening to the stream!",
            id
        );
    }

    Ok(StatusCode::OK)
}

pub async fn handle_stream(
    Extension(tx): Extension<RoomsStream>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = tx.subscribe();
    let stream = BroadcastStream::new(rx);

    Sse::new(stream.map(|_msg| Event::default().event("reload")).map(Ok)).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(600))
            .text("keep-alive-text"),
    )
}

// TODO: Add real error handling
pub async fn room(State(state): State<AppState>, Path(id): Path<i32>) -> impl IntoResponse {
    let room = sqlx::query_as::<_, Room>("SELECT * FROM rooms WHERE id=$1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .unwrap();
    templates::RoomViewTemplate { room }
}

pub async fn send_message(
    State(state): State<AppState>,
    Path(room): Path<i32>,
    Form(form): Form<MessageNew>,
) -> impl IntoResponse {
    let message = sqlx::query_as::<_, Message>("INSERT INTO messages (sent_date, contents) VALUES ($1, $2) RETURNING id, sent_date, contents")
        .bind(Local::now())
        .bind(form.contents)
        .fetch_one(&state.db)
        .await
        .unwrap();
    if let Err(e) = sqlx::query("INSERT INTO room_messages (room, message) VALUES ($1, $2)")
        .bind(room)
        .bind(message.id)
        .execute(&state.db)
        .await
    {
        log::error!("Error while creating message relation: {e}");
    };
    templates::MessageTemplate { message }
}
