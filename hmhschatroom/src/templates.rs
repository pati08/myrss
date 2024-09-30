use crate::models;
use askama::Template;

fn format_datetime(datetime: &chrono::DateTime<chrono::Local>) -> String {
    let now = chrono::Local::now();
    let is_today = now.date_naive() == datetime.date_naive();
    if is_today {
        datetime.format("%I:%M %P").to_string()
    } else {
        datetime.format("%d %b, %Y - %I:%M %P").to_string()
    }
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct HelloTemplate;

#[derive(Template)]
#[template(path = "rooms.html")]
pub struct Records {
    pub rooms: Vec<models::Room>,
}

#[derive(Template)]
#[template(path = "room.html")]
pub struct RoomNewTemplate {
    pub room: models::Room,
}

#[derive(Template)]
#[template(path = "view-room.html")]
pub struct RoomViewTemplate {
    pub room: models::Room,
}

#[derive(Template)]
#[template(path = "message.html")]
pub struct MessageTemplate {
    pub message: models::Message,
}

#[derive(Template)]
#[template(path = "messages.html")]
pub struct MessagesTemplate {
    pub messages: Vec<models::Message>,
}
