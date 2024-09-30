use crate::models::{Message, Room};
use sqlx::{
    types::chrono::{DateTime, Local},
    PgPool, Row,
};
use std::collections::HashMap;

pub async fn get_rooms_with_messages(db: &PgPool) -> Result<Vec<Room>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT r.id AS room_id, r.name, r.description, 
               m.id AS message_id, m.sent_date, m.contents
        FROM rooms r
        LEFT JOIN room_messages rm ON r.id = rm.room
        LEFT JOIN messages m ON rm.message = m.id
        ORDER BY r.id, m.id;
        "#,
    )
    .fetch_all(db)
    .await?;

    // Use a HashMap to group messages by room
    let mut rooms_map: HashMap<i32, Room> = HashMap::new();

    for row in rows {
        let room_id: i32 = row.try_get("room_id")?;
        let room_name: String = row.try_get("name")?;
        let room_description: String = row.try_get("description")?;

        // If the room already exists in the map, we just push a new message into it.
        let room = rooms_map.entry(room_id).or_insert(Room {
            id: room_id,
            name: room_name,
            description: room_description,
            messages: Vec::new(),
        });

        // Handle optional message (since LEFT JOIN can return NULL for the message part)
        if let Ok(message_id) = row.try_get::<i32, _>("message_id") {
            let message_sent_date: DateTime<Local> = row.try_get("sent_date")?;
            let message_contents: String = row.try_get("contents")?;

            room.messages.push(Message {
                id: message_id,
                sent_date: message_sent_date,
                contents: message_contents,
            });
        }
    }

    // Collect the rooms into a Vec and return
    let mut rooms: Vec<Room> = rooms_map.into_values().collect();

    // Sort messages in each room by message id (if needed, SQL already orders by id)
    for room in &mut rooms {
        room.messages.sort_unstable_by_key(|message| message.id);
        room.messages.reverse();
    }

    // Return the rooms ordered by their id
    rooms.sort_unstable_by_key(|room| room.id);

    Ok(rooms)
}

pub async fn get_room_by_id(pool: &PgPool, room_id: i32) -> Result<Option<Room>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT r.id AS room_id, r.name, r.description, 
               m.id AS message_id, m.sent_date, m.contents
        FROM rooms r
        LEFT JOIN room_messages rm ON r.id = rm.room
        LEFT JOIN messages m ON rm.message = m.id
        WHERE r.id = $1
        ORDER BY m.id;
        "#,
    )
    .bind(room_id) // Bind the room_id to the query parameter
    .fetch_all(pool)
    .await?;

    // If no rows are returned, the room doesn't exist
    if rows.is_empty() {
        return Ok(None);
    }

    // Use a single Room object since we are only querying one room
    let mut room: Option<Room> = None;

    for row in rows {
        // Since all rows should have the same room information, set it once
        if room.is_none() {
            let room_name: String = row.try_get("name")?;
            let room_description: String = row.try_get("description")?;
            room = Some(Room {
                id: room_id,
                name: room_name,
                description: room_description,
                messages: Vec::new(),
            });
        }

        // Handle optional message (since LEFT JOIN can return NULL for the message part)
        if let Ok(message_id) = row.try_get::<i32, _>("message_id") {
            let message_sent_date: DateTime<Local> = row.try_get("sent_date")?;
            let message_contents: String = row.try_get("contents")?;

            if let Some(ref mut room) = room {
                room.messages.push(Message {
                    id: message_id,
                    sent_date: message_sent_date,
                    contents: message_contents,
                });
            }
        }
    }

    // If there are messages, sort them by message id (lowest first)
    if let Some(ref mut room) = room {
        room.messages.sort_unstable_by_key(|message| message.id);
        room.messages.reverse();
    }

    Ok(room)
}

pub async fn get_room_messages(pool: &PgPool, room_id: i32) -> Result<Vec<Message>, sqlx::Error> {
    sqlx::query_as::<_, Message>(
        r#"SELECT m.id, m.sent_date, m.contents
        FROM messages m
        JOIN room_messages rm ON m.id = rm.message
        JOIN rooms r ON rm.room = r.id
        WHERE r.id = $1
        ORDER BY m.sent_date DESC"#,
    )
    .bind(room_id)
    .fetch_all(pool)
    .await
}
