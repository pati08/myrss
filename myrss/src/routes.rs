use axum::{
    response::{sse::Event, IntoResponse, Redirect, Sse},
    Extension, Form,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use futures::Stream;
use serde::Deserialize;
use sqlx::types::chrono::Local;
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
    let jar = jar.add(Cookie::new("sender-name", payload.name));
    (jar, Redirect::to("/feed"))
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
                sent_date: Local::now(),
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
    let name = name.value().to_string();

    let rx = tx.subscribe();
    let stream = StreamWrapper(name.clone(), tx.clone(), BroadcastStream::new(rx));

    let sse = Sse::new(stream.filter_map(|msg| msg.ok()).map(|msg| {
        Result::<_, Infallible>::Ok(
            Event::default().data(MessageTemplate { message: msg }.to_string()),
        )
    }))
    .keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keep-alive-text"),
    );
    if tx
        .send(Message {
            sender: "System".to_string(),
            sent_date: Local::now(),
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
    let sender = jar.get("sender-name").unwrap().value().to_string();
    let message = Message {
        sender,
        sent_date: Local::now(),
        contents: form.contents,
    };
    if tx.send(message.clone()).is_err() {
        println!("Message sent but nobody listening to the stream");
    }
    templates::MessageTemplate { message }
}

pub async fn feed(jar: CookieJar) -> impl IntoResponse {
    if jar.get("sender-name").is_none() {
        return Redirect::to("/").into_response();
    }
    templates::FeedTemplate.into_response()
}
