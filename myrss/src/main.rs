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

const DEFAULT_PORT: u16 = 3000;

#[cfg(not(feature = "shuttle"))]
#[tokio::main]
async fn main() {
    use std::net::Ipv4Addr;

    env_logger::init();

    let groq_api_key = option_env!("GROQ_API_KEY")
        .map(|v| v.to_string())
        .or_else(|| std::env::var("GROQ_API_KEY").ok())
        .expect("No Groq API key available");

    let addr = match std::env::var("RSS_DO_NOT_PUBLISH") {
        Ok(s) if s == "1" => Ipv4Addr::new(127, 0, 0, 1),
        _ => Ipv4Addr::new(0, 0, 0, 0),
    };
    let port = match std::env::var("SERVER_PORT").map(|v| v.parse::<u16>()) {
        Ok(Ok(port)) => port,
        _ => DEFAULT_PORT,
    };
    let router = router::init_router(groq_api_key).await;
    let listener = tokio::net::TcpListener::bind((addr, port)).await.unwrap();

    axum::serve(listener, router).await.unwrap();
}
