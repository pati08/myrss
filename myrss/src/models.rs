use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Local};

#[derive(sqlx::FromRow, Serialize, Deserialize, Clone)]
pub struct Message {
    pub sender: String,
    pub sent_date: DateTime<Local>,
    pub contents: String,
}
#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct MessageNew {
    pub contents: String,
}
