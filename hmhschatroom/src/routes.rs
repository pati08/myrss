use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Response, Sse},
    Extension, Form,
};
use axum_extra::extract::cookie::CookieJar;
use sqlx::types::chrono::Local;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt as _};

use crate::{
    db,
    models::{Message, MessageNew, Room, RoomNew, RoomRaw},
};
use crate::{errors::ApiError, router::AppState, router::RoomsStream, templates};

pub async fn home() -> impl IntoResponse {
    templates::HelloTemplate
}

pub async fn fetch_rooms(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let rooms = crate::db::get_rooms_with_messages(&state.db)
        .await
        .map_err(ApiError::SQLError)?;

    Ok(templates::Records { rooms })
}

pub async fn styles() -> Result<impl IntoResponse, ApiError> {
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/css")
        .body(include_str!("../templates/styles.css").to_owned())?;

    Ok(response)
}

pub async fn tailwind() -> Result<impl IntoResponse, ApiError> {
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/css")
        .body(include_str!("../templates/tailwind.css").to_owned())?;

    Ok(response)
}

pub async fn create_room(
    State(state): State<AppState>,
    Form(form): Form<RoomNew>,
) -> impl IntoResponse {
    println!("Creating room");
    let room = sqlx::query_as::<_, RoomRaw>(
        "INSERT INTO rooms (name, description) VALUES ($1, $2) RETURNING id, name, description",
    )
    .bind(form.name)
    .bind(form.description)
    .fetch_one(&state.db)
    .await
    .unwrap();

    let room = Room {
        id: room.id,
        name: room.name,
        description: room.description,
        messages: Vec::new(),
    };
    templates::RoomNewTemplate { room }
}

pub async fn delete_room(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    sqlx::query("DELETE FROM rooms WHERE ID = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::OK)
}

pub async fn handle_stream(
    Extension(tx): Extension<RoomsStream>,
    Path(room): Path<i32>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = tx.subscribe();
    let stream = BroadcastStream::new(rx);

    Sse::new(
        stream
            .filter(move |msg| msg.as_ref().is_ok_and(move |msg| *msg == room))
            .map(|_msg| Ok(Event::default().event("reload").data("\n"))),
    )
    .keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keep-alive-text"),
    )
}

// TODO: Add real error handling
pub async fn room(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    let Some(room) = crate::db::get_room_by_id(&state.db, id)
        .await
        .map_err(ApiError::SQLError)?
    else {
        return Err(ApiError::DoesNotExist);
    };

    Ok(templates::RoomViewTemplate { room })
}

pub async fn send_message(
    Extension(tx): Extension<RoomsStream>,
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
    if tx.send(room).is_err() {
        println!("Message sent but nobody listening to the stream");
    }
    templates::MessageTemplate { message }
}

pub async fn messages(State(state): State<AppState>, Path(room): Path<i32>) -> impl IntoResponse {
    let messages = db::get_room_messages(&state.db, room).await.unwrap();
    templates::MessagesTemplate { messages }
}

pub async fn sign_in_page(cookies: CookieJar) -> impl IntoResponse {
    //
}
