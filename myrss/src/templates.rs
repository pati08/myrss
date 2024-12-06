use crate::models;
use askama::Template;
use chrono::FixedOffset;

fn format_datetime(datetime: &chrono::DateTime<chrono::Utc>, tz: &i32) -> String {
    let now = chrono::Utc::now();
    let is_today = now.date_naive() == datetime.date_naive();
    let offset = FixedOffset::west_opt(tz * 60).unwrap();
    let datetime = datetime.with_timezone(&offset);
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
#[template(path = "message.html", escape = "none")]
pub struct MessageTemplate {
    pub message: models::Message,
    pub tz: i32,
}

#[derive(Template)]
#[template(path = "feed.html")]
pub struct FeedTemplate;

#[derive(Template)]
#[template(path = "banned-name.html")]
pub struct BannedName {
    pub name: String,
}
