mod ai;
mod errors;
mod models;
mod router;
mod routes;
mod templates;

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
