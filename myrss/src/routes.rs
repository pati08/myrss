use axum::{
    extract::State,
    http::{HeaderValue, StatusCode},
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
use tokio::sync::broadcast::Sender;
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream};
use tokio_stream::StreamExt as _;

use crate::{
    models::{Message, MessageNew},
    router::AppState,
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
        let num_receivers = self.1.receiver_count();
        send_message_backend(
            self.1.clone(),
            construct_message(
                format!("{} left. Users currently online: {num_receivers}", self.0),
                "System",
                false,
            ),
        );
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
        let preview = if msg.contents.len() <= 40 {
            msg.contents.clone()
        } else {
            format!("{}...", &msg.contents[..37])
        };
        let should_notify = msg.should_notify;
        let msghtml = MessageTemplate { message: msg, tz }.to_string();
        let data = json!({
            "sender": sname,
            "message": msghtml,
            "preview": preview,
            "notify": should_notify,
        });
        Result::<_, Infallible>::Ok(Event::default().data(data.to_string()))
    }))
    .keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keep-alive-text"),
    );
    let num_receivers = tx.receiver_count();
    send_message_backend(
        tx,
        construct_message(
            format!("{name} joined. Users currently online: {num_receivers}",),
            "System",
            true,
        ),
    );
    let mut resp = sse.into_response();
    resp.headers_mut()
        .append("X-Accel-Buffering", HeaderValue::from_static("no"));
    resp
}

pub async fn send_message(
    state: State<AppState>,
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
    let message = construct_message(form.contents.clone(), sender.clone(), true);
    let tmsg = message.clone();

    if form.contents.chars().next().is_some_and(|c| c == '!') {
        log::debug!("Detected message to ai, handling");
        let tx = tx.clone();
        tokio::spawn(async move {
            let messages = {
                let mut messages = state.ai_messages.lock().unwrap();
                messages.push(super::create_user_messsage(form.contents[1..].to_string()));
                if messages.len() > 10 {
                    *messages = messages[(messages.len() - 10)..].to_vec();
                }
                messages.clone()
            };

            let mut request_args = async_openai::types::CreateChatCompletionRequestArgs::default();
            request_args.messages(messages);
            request_args.model("llama-3.3-70b-versatile");

            let response = state
                .ai_client
                .chat()
                .create(request_args.build().unwrap())
                .await;

            match response {
                Ok(response) => {
                    let msg = construct_message(
                        response.choices[0]
                            .message
                            .content
                            .clone()
                            .unwrap_or_default(),
                        "AI",
                        true,
                    );
                    if tx.send(msg).is_ok() {
                        state
                            .ai_messages
                            .lock()
                            .unwrap()
                            .push(super::create_assistant_messsage(
                                response.choices[0]
                                    .message
                                    .content
                                    .clone()
                                    .unwrap_or_default(),
                            ));
                    }
                }

                Err(e) => log::error!("Failed to get AI response: {e}"),
            }
        });
    };

    // INFO: This is an attempt to mitigate some long server response times I
    // noticed.
    // TODO: Work more on this
    tokio::task::spawn_blocking(move || {
        send_message_backend(tx, tmsg);
    });
    templates::MessageTemplate { message, tz }.into_response()
}
fn construct_message(contents: impl ToString, sender: impl ToString, notify: bool) -> Message {
    let contents = contents.to_string();
    let contents = ammonia::clean(&contents);
    let contents = contents
        .lines()
        .map(|line| line.trim_end()) // Trim trailing spaces
        .collect::<Vec<_>>() // Collect into a Vec
        .join("  \n"); // Join with Markdown's line break syntax (two spaces + newline)

    let contents = markdown::to_html(&contents); // Convert to HTML from Markdown
    let sender = sender.to_string();
    Message {
        sender,
        contents,
        sent_date: Utc::now(),
        should_notify: notify,
    }
}
fn send_message_backend(tx: Sender<Message>, message: Message) {
    if tx.send(message).is_err() {
        log::warn!("Nobody is listening to the stream");
    }
}

pub async fn feed(jar: CookieJar) -> impl IntoResponse {
    if jar.get("sender-name").is_none() {
        return Redirect::to("/").into_response();
    }
    templates::FeedTemplate.into_response()
}
