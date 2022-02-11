use chrono::Utc;

pub fn get_current_time() -> String {
    let tz = chrono_tz::Asia::Tokyo;
    let datetime = Utc::now().with_timezone(&tz);
    datetime.to_string()
}
