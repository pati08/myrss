use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Local};
//
// #[derive(Clone, Serialize, Debug)]
// pub enum MutationKind {
//     Create,
//     Delete,
// }
//
// #[derive(Clone, Serialize, Debug)]
// pub struct TodoUpdate {
//     pub mutation_kind: MutationKind,
//     pub id: i32,
// }

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct Room {
    pub id: i32,
    pub name: String,
    pub description: String,
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct RoomNew {
    pub name: String,
    pub description: String,
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct Message {
    pub id: i32,
    pub sent_date: DateTime<Local>,
    pub contents: String,
}
#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct MessageNew {
    pub contents: String,
}
