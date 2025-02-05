mod errors;
mod models;
mod router;
mod routes;
mod templates;

use async_openai::types::ChatCompletionRequestMessage;

fn create_user_messsage(message: String, name: String) -> ChatCompletionRequestMessage {
    let message = format!("\"{name}\" says:\n----------\n{message}");
    ChatCompletionRequestMessage::User(async_openai::types::ChatCompletionRequestUserMessage {
        content: async_openai::types::ChatCompletionRequestUserMessageContent::Text(message),
        name: Some(name),
    })
}

fn create_system_messsage(message: String) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::System(async_openai::types::ChatCompletionRequestSystemMessage {
        content: async_openai::types::ChatCompletionRequestSystemMessageContent::Text(message),
        name: None,
    })
}

fn create_assistant_messsage(message: String) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::Assistant(
        async_openai::types::ChatCompletionRequestAssistantMessage {
            content: Some(
                async_openai::types::ChatCompletionRequestAssistantMessageContent::Text(message),
            ),
            audio: None,
            refusal: None,
            name: None,
            tool_calls: None,
            function_call: None,
        },
    )
}

#[cfg(feature = "shuttle")]
#[shuttle_runtime::main]
async fn main(
    #[shuttle_runtime::Secrets] secrets: shuttle_runtime::SecretStore,
) -> shuttle_axum::ShuttleAxum {
    let groq_api_key = secrets.get("GROQ_API_KEY").unwrap();
    let router = router::init_router(groq_api_key).await;

    Ok(router.into())
}

#[cfg(not(feature = "shuttle"))]
#[tokio::main]
async fn main() {
    env_logger::init();
    let groq_api_key = option_env!("GROQ_API_KEY")
        .map(|v| v.to_string())
        .or_else(|| std::env::var("GROQ_API_KEY").ok())
        .expect("No Groq API key available");
    let router = router::init_router(groq_api_key).await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
