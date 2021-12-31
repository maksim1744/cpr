use chrono::{DateTime, NaiveDateTime, Utc};

pub fn get_time(offset: i64) -> String {
    let dt = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(Utc::now().timestamp() + offset, 0), Utc);
    dt.format("%H:%M:%S").to_string()
}

pub fn get_date(offset: i64) -> String {
    let dt = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(Utc::now().timestamp() + offset, 0), Utc);
    dt.format("%Y-%m-%d").to_string()
}

pub fn get_datetime(offset: i64) -> String {
    let dt = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(Utc::now().timestamp() + offset, 0), Utc);
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}
