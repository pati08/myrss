use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::DateTime;

#[derive(sqlx::FromRow, Serialize, Deserialize, Clone)]
pub struct Message {
    pub sender: String,
    pub sent_date: DateTime<Utc>,
    pub contents: String,
}
#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct MessageNew {
    pub contents: String,
}
