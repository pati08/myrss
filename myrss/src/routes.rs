use axum::{
    http::StatusCode,
    response::{sse::Event, IntoResponse, Redirect, Sse},
    Extension, Form,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use chrono::Utc;
use futures::Stream;
use serde::Deserialize;
use serde_json::json;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream};
use tokio_stream::StreamExt as _;

use crate::{
    models::{Message, MessageNew},
    templates::MessageTemplate,
};
use crate::{router::RoomsStream, templates};

pub async fn home(jar: CookieJar) -> impl IntoResponse {
    if jar.get("sender-name").is_some() {
        return Redirect::to("/feed").into_response();
    }
    templates::StartTemplate.into_response()
}

#[derive(Deserialize)]
pub struct NamePayload {
    name: String,
}

pub async fn set_name(jar: CookieJar, Form(payload): Form<NamePayload>) -> impl IntoResponse {
    // let headers = AppendHeaders([(SET_COOKIE, format!("sender-name={}", payload.name))]);
    let name = payload.name.trim();
    if name.to_lowercase() == "system" {
        // If the user tries to pretend to be the System, don't let them
        return (
            StatusCode::BAD_REQUEST,
            templates::BannedName {
                name: name.to_string(),
            },
        )
            .into_response();
    }
    let jar = jar.add(Cookie::new("sender-name", payload.name));
    (jar, Redirect::to("/feed")).into_response()
}

struct StreamWrapper(
    String,
    tokio::sync::broadcast::Sender<Message>,
    BroadcastStream<Message>,
);
impl Drop for StreamWrapper {
    fn drop(&mut self) {
        if self
            .1
            .send(Message {
                sender: "System".to_string(),
                sent_date: Utc::now(),
                contents: format!(
                    "{} left. Users currently online: {}",
                    self.0,
                    self.1.receiver_count() - 1
                ),
            })
            .is_err()
        {
            log::warn!("Nobody heard left message");
        }
    }
}

impl Stream for StreamWrapper {
    type Item = Result<Message, BroadcastStreamRecvError>;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let inner_stream = unsafe { self.map_unchecked_mut(|wrapper| &mut wrapper.2) };
        inner_stream.poll_next(cx)
    }
}

pub async fn handle_stream(
    jar: CookieJar,
    // State(count): State<Arc<Mutex<u32>>>,
    Extension(tx): Extension<RoomsStream>,
) -> impl IntoResponse {
    let Some(name) = jar.get("sender-name") else {
        return Redirect::to("/").into_response();
    };
    let Some(tz) = jar.get("timezone") else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(tz) = tz.value().to_string().parse::<i32>() else {
        return (jar.remove("timezone"), Redirect::to("/")).into_response();
    };
    let name = name.value().to_string();

    let rx = tx.subscribe();
    let stream = StreamWrapper(name.clone(), tx.clone(), BroadcastStream::new(rx));

    let sse = Sse::new(stream.filter_map(|msg| msg.ok()).map(move |msg| {
        let sname = msg.sender.clone();
        let msghtml = MessageTemplate { message: msg, tz }.to_string();
        let data = json!({
            "sender": sname,
            "message": msghtml,
        });
        Result::<_, Infallible>::Ok(Event::default().data(data.to_string()))
    }))
    .keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keep-alive-text"),
    );
    if tx
        .send(Message {
            sender: "System".to_string(),
            sent_date: Utc::now(),
            contents: format!(
                "{name} joined. Users currently online: {}",
                tx.receiver_count()
            ),
        })
        .is_err()
    {
        log::warn!("Nobody received join notification");
    };
    sse.into_response()
}

#[axum::debug_handler]
pub async fn send_message(
    Extension(tx): Extension<RoomsStream>,
    jar: CookieJar,
    Form(form): Form<MessageNew>,
) -> impl IntoResponse {
    let Some(sender) = jar.get("sender-name") else {
        return Redirect::to("/").into_response();
    };
    let Some(tz) = jar.get("timezone") else {
        return Redirect::to("/").into_response();
    };
    let Ok(tz) = tz.value().to_string().parse::<i32>() else {
        return (jar.remove("timezone"), Redirect::to("/")).into_response();
    };
    let sender = sender.value().to_string();
    let message = Message {
        sender,
        sent_date: Utc::now(),
        contents: form.contents,
    };
    let tmsg = message.clone();
    // INFO: This is an attempt to mitigate some long server response times I
    // noticed
    tokio::task::spawn_blocking(move || {
        if tx.send(tmsg).is_err() {
            log::warn!("Message sent but nobody listening to the stream");
        }
    });
    templates::MessageTemplate { message, tz }.into_response()
}

pub async fn feed(jar: CookieJar) -> impl IntoResponse {
    if jar.get("sender-name").is_none() {
        return Redirect::to("/").into_response();
    }
    templates::FeedTemplate.into_response()
}
