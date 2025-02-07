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
use thiserror::Error;
use tokio::sync::broadcast::Sender;
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream};
use tokio_stream::StreamExt as _;

use crate::{
    ai::Bot,
    models::{Message, MessageNew},
    router::AppState,
    templates::MessageTemplate,
};
use crate::{router::RoomsStream, templates};

const HELP_MESSAGE: &str = "Valid commands:
- !ai &lt;message&gt; - ask a question to the default bot (greg)
- !ask &lt;bot&gt; &lt;message&gt; - ask a question to a bot by name
- !newbot &lt;name&gt; [lang:&lt;language&gt;] &lt;instructions&gt; - create a new
bot that follows custom instructions
- !listbots - list bots by name
- !removebot <bot> - remove a bot (you can only remove a bot you created)
- !online - ask how many users are online
- !help - show this message";

const BOT_RESPONSES_NOTIFY: bool = false;

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
            format!("{}...", msg.contents.chars().take(37).collect::<String>())
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

    match parse_message_command(&form.contents) {
        Some(Ok(MessageCommand::NumUsersOnlineQuery)) => {
            let num_receivers = tx.receiver_count();
            send_message_delayed_backend(
                tx.clone(),
                construct_message(
                    format!("Users currently online: {num_receivers}"),
                    "Server",
                    false,
                ),
            );
        }
        Some(Ok(MessageCommand::AIQuery { bot, query })) => {
            let tx = tx.clone();
            tokio::spawn(async move {
                let ai_context = state.0.ai_context;
                let response = ai_context
                    .get_response(&query, &sender, bot.as_deref())
                    .await;
                if let Ok(response) = response {
                    let message = construct_message(
                        response.response,
                        format!("{} (Bot)", response.bot_name),
                        BOT_RESPONSES_NOTIFY,
                    );
                    send_message_backend(tx, message);
                }
            });
        }
        Some(Ok(MessageCommand::Help)) => {
            send_message_delayed_backend(
                tx.clone(),
                construct_message(HELP_MESSAGE, "Server", false),
            );
        }
        Some(Ok(MessageCommand::AICreate { name, lang, config })) => {
            send_message_delayed_backend(
                tx.clone(),
                construct_message("New bot created.", "System", false),
            );
            state
                .0
                .ai_context
                .add_bot(Bot::new(name, sender, Some(config), lang))
                .await;
        }
        Some(Ok(MessageCommand::AIList)) => {
            let bots_list = state
                .0
                .ai_context
                .bots()
                .await
                .into_iter()
                .map(|i| format!("- {}", i.name()))
                .collect::<Vec<_>>()
                .join("\n");
            send_message_delayed_backend(
                tx.clone(),
                construct_message(format!("Bots online:\n{bots_list}"), "System", false),
            );
        }
        Some(Ok(MessageCommand::RemoveBot { bot })) => {
            if state.0.ai_context.remove_bot_by_name(bot).await.is_some() {
                send_message_delayed_backend(
                    tx.clone(),
                    construct_message("Bot removed.", "System", false),
                );
            } else {
                send_message_delayed_backend(
                    tx.clone(),
                    construct_message("There is no bot by that name.", "System", false),
                );
            }
        }
        Some(Err(_)) => {
            let message = form.contents.clone();
            send_message_delayed_backend(
                tx.clone(),
                construct_message(
                    format!(
                        "Invalid command `{}`. Use !help to list valid commands",
                        message
                    ),
                    "Server",
                    false,
                ),
            );
        }
        None => {}
    }

    if form.contents.chars().next().is_some_and(|c| c == '!') {};

    // INFO: This is an attempt to mitigate some long server response times I
    // noticed.
    // TODO: Work more on this
    send_message_backend(tx, tmsg);
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

fn send_message_delayed_backend(tx: Sender<Message>, message: Message) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(250)).await;
        if tx.send(message).is_err() {
            log::warn!("Nobody is listening to the stream");
        }
    });
}

pub async fn feed(jar: CookieJar) -> impl IntoResponse {
    if jar.get("sender-name").is_none() {
        return Redirect::to("/").into_response();
    }
    templates::FeedTemplate.into_response()
}

enum MessageCommand {
    AIQuery {
        bot: Option<String>,
        query: String,
    },
    AICreate {
        name: String,
        lang: Option<String>,
        config: String,
    },
    RemoveBot {
        bot: String,
    },
    AIList,
    NumUsersOnlineQuery,
    Help,
}

#[derive(Debug, Error)]
enum MessageParseError {
    #[error("Invalid command or command syntax entered")]
    InvalidCommand,
}

fn parse_message_command(s: &str) -> Option<Result<MessageCommand, MessageParseError>> {
    let Some('!') = s.chars().next() else {
        return None;
    };
    let command_input = &s[1..];
    let Some(command) = command_input.split_whitespace().next() else {
        return Some(Err(MessageParseError::InvalidCommand));
    };
    if command == "ai" {
        Some(Ok(MessageCommand::AIQuery {
            bot: None,
            query: command_input[3..].to_string(),
        }))
    } else if command == "ask" && command_input.split_whitespace().count() > 2 {
        let bot_name = command_input.split_whitespace().nth(1).unwrap();
        let query = command_input
            .chars()
            .skip(3)
            .skip_while(|c| c.is_whitespace())
            .skip(bot_name.len())
            .skip_while(|c| c.is_whitespace())
            .collect();
        Some(Ok(MessageCommand::AIQuery {
            bot: Some(bot_name.to_string()),
            query,
        }))
    } else if command == "online" {
        Some(Ok(MessageCommand::NumUsersOnlineQuery))
    } else if command == "help" {
        Some(Ok(MessageCommand::Help))
    } else if command == "newbot" && command_input.split_whitespace().count() > 2 {
        let name = command_input.split_whitespace().nth(1).unwrap();
        let lang_word = command_input.split_whitespace().nth(2).unwrap();
        let lang = if lang_word.starts_with("lang=") {
            Some(lang_word[5..].to_string())
        } else {
            None
        };
        let customizations = command_input
            .chars()
            .skip(6)
            .skip_while(|c| c.is_whitespace())
            .skip(name.len())
            .skip_while(|c| c.is_whitespace());
        let customizations_final: String = if let Some(ref lang) = lang {
            customizations
                .skip(lang.len() + 5)
                .skip_while(|c| c.is_whitespace())
                .collect()
        } else {
            customizations.collect()
        };
        Some(Ok(MessageCommand::AICreate {
            name: name.to_string(),
            lang,
            config: customizations_final,
        }))
    } else if command == "listbots" {
        Some(Ok(MessageCommand::AIList))
    } else if command == "removebot" {
        Some(Ok(MessageCommand::RemoveBot {
            bot: command_input[10..].to_string(),
        }))
    } else {
        Some(Err(MessageParseError::InvalidCommand))
    }
}
