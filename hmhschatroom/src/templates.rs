use crate::models;
use askama::Template;

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
