use chrono::{DateTime, Utc};

pub fn get_time(offset: i64) -> String {
    let dt = DateTime::<Utc>::from_naive_utc_and_offset(
        DateTime::from_timestamp(Utc::now().timestamp() + offset, 0)
            .unwrap()
            .naive_utc(),
        Utc,
    );
    dt.format("%H:%M:%S").to_string()
}

pub fn get_date(offset: i64) -> String {
    let dt = DateTime::<Utc>::from_naive_utc_and_offset(
        DateTime::from_timestamp(Utc::now().timestamp() + offset, 0)
            .unwrap()
            .naive_utc(),
        Utc,
    );
    dt.format("%Y-%m-%d").to_string()
}

pub fn get_datetime(offset: i64) -> String {
    let dt = DateTime::<Utc>::from_naive_utc_and_offset(
        DateTime::from_timestamp(Utc::now().timestamp() + offset, 0)
            .unwrap()
            .naive_utc(),
        Utc,
    );
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}
