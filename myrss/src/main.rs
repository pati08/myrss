mod errors;
mod models;
mod router;
mod routes;
mod templates;

#[shuttle_runtime::main]
async fn main(
    #[shuttle_runtime::Secrets] secrets: shuttle_runtime::SecretStore,
) -> shuttle_axum::ShuttleAxum {
    let groq_api_key = secrets.get("GROQ_API_KEY").unwrap();
    let router = router::init_router(groq_api_key).await;

    Ok(router.into())
}
