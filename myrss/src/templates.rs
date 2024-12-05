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
pub struct StartTemplate;

#[derive(Template)]
#[template(path = "message.html")]
pub struct MessageTemplate {
    pub message: models::Message,
}

#[derive(Template)]
#[template(path = "feed.html")]
pub struct FeedTemplate;

#[derive(Template)]
#[template(path = "banned-name.html")]
pub struct BannedName {
    pub name: String,
}
