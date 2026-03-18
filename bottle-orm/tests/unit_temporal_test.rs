use bottle_orm::{
    database::Drivers,
    temporal::{
        format_datetime_fixed_for_driver, format_datetime_for_driver,
        format_naive_datetime_for_driver, get_postgres_type_cast, is_temporal_type,
        parse_datetime_fixed, parse_datetime_utc, parse_naive_date, parse_naive_datetime,
        parse_naive_time,
    },
};
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

// ============================================================================
// parse_datetime_utc
// ============================================================================

#[test]
fn test_parse_datetime_utc_rfc3339() {
    let result = parse_datetime_utc("2024-06-15T12:30:00Z");
    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.year(), 2024);
    assert_eq!(dt.month(), 6);
    assert_eq!(dt.day(), 15);
    assert_eq!(dt.hour(), 12);
    assert_eq!(dt.minute(), 30);
}

#[test]
fn test_parse_datetime_utc_with_offset() {
    // Should convert FixedOffset to UTC
    let result = parse_datetime_utc("2024-01-01T10:00:00+02:00");
    assert!(result.is_ok());
    let dt = result.unwrap();
    // +02:00 means UTC = 10:00 - 2:00 = 08:00
    assert_eq!(dt.hour(), 8);
}

#[test]
fn test_parse_datetime_utc_mysql_format() {
    // MySQL/SQLite "YYYY-MM-DD HH:MM:SS.f"
    let result = parse_datetime_utc("2024-03-20 14:45:30.123456");
    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.year(), 2024);
    assert_eq!(dt.month(), 3);
    assert_eq!(dt.day(), 20);
    assert_eq!(dt.hour(), 14);
}

#[test]
fn test_parse_datetime_utc_no_subseconds() {
    let result = parse_datetime_utc("2024-03-20 14:45:30");
    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.second(), 30);
}

#[test]
fn test_parse_datetime_utc_invalid() {
    let result = parse_datetime_utc("not-a-date");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("not-a-date"));
}

// ============================================================================
// parse_datetime_fixed
// ============================================================================

#[test]
fn test_parse_datetime_fixed_rfc3339() {
    let result = parse_datetime_fixed("2024-06-15T12:30:00+03:00");
    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.hour(), 12);
    assert_eq!(dt.offset().local_minus_utc(), 3 * 3600);
}

#[test]
fn test_parse_datetime_fixed_naive_fallback() {
    // No timezone info -> assume UTC (+00:00)
    let result = parse_datetime_fixed("2024-01-10 09:15:00.000");
    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.offset().local_minus_utc(), 0);
}

#[test]
fn test_parse_datetime_fixed_invalid() {
    let result = parse_datetime_fixed("garbage");
    assert!(result.is_err());
}

// ============================================================================
// parse_naive_datetime
// ============================================================================

#[test]
fn test_parse_naive_datetime_space_format() {
    let result = parse_naive_datetime("2024-07-04 08:00:00");
    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.year(), 2024);
    assert_eq!(dt.month(), 7);
    assert_eq!(dt.day(), 4);
}

#[test]
fn test_parse_naive_datetime_with_subseconds() {
    let result = parse_naive_datetime("2024-07-04 08:00:00.999");
    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.second(), 0);
}

#[test]
fn test_parse_naive_datetime_t_separator() {
    let result = parse_naive_datetime("2024-12-31T23:59:59.000");
    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.hour(), 23);
    assert_eq!(dt.minute(), 59);
}

#[test]
fn test_parse_naive_datetime_no_seconds() {
    // HH:MM without seconds
    let result = parse_naive_datetime("2024-01-01 12:30");
    assert!(result.is_ok());
    let dt = result.unwrap();
    assert_eq!(dt.hour(), 12);
    assert_eq!(dt.minute(), 30);
}

#[test]
fn test_parse_naive_datetime_invalid() {
    let result = parse_naive_datetime("2024-13-40 99:99:99");
    assert!(result.is_err());
}

// ============================================================================
// parse_naive_date
// ============================================================================

#[test]
fn test_parse_naive_date_valid() {
    let result = parse_naive_date("2024-02-29");
    assert!(result.is_ok()); // 2024 is a leap year
    let d = result.unwrap();
    assert_eq!(d.month(), 2);
    assert_eq!(d.day(), 29);
}

#[test]
fn test_parse_naive_date_invalid_leap() {
    // 2023 is not a leap year
    let result = parse_naive_date("2023-02-29");
    assert!(result.is_err());
}

#[test]
fn test_parse_naive_date_invalid_format() {
    let result = parse_naive_date("not-a-date");
    assert!(result.is_err());
}

// ============================================================================
// parse_naive_time
// ============================================================================

#[test]
fn test_parse_naive_time_valid() {
    let result = parse_naive_time("14:30:00");
    assert!(result.is_ok());
    let t = result.unwrap();
    assert_eq!(t.hour(), 14);
    assert_eq!(t.minute(), 30);
    assert_eq!(t.second(), 0);
}

#[test]
fn test_parse_naive_time_with_subseconds() {
    let result = parse_naive_time("09:05:30.123456");
    assert!(result.is_ok());
    let t = result.unwrap();
    assert_eq!(t.hour(), 9);
    assert_eq!(t.minute(), 5);
    assert_eq!(t.second(), 30);
}

#[test]
fn test_parse_naive_time_invalid() {
    let result = parse_naive_time("25:00:00");
    assert!(result.is_err());
}

// ============================================================================
// format_datetime_for_driver
// ============================================================================

#[test]
fn test_format_datetime_postgres_is_rfc3339() {
    let dt: DateTime<Utc> = Utc.with_ymd_and_hms(2024, 1, 15, 10, 20, 30).unwrap();
    let s = format_datetime_for_driver(&dt, &Drivers::Postgres);
    // RFC 3339 should contain 'T' separator and timezone
    assert!(s.contains('T'));
    assert!(s.contains('+') || s.ends_with('Z'));
}

#[test]
fn test_format_datetime_sqlite_is_rfc3339() {
    let dt: DateTime<Utc> = Utc.with_ymd_and_hms(2024, 1, 15, 10, 20, 30).unwrap();
    let s = format_datetime_for_driver(&dt, &Drivers::SQLite);
    assert!(s.contains('T'));
}

#[test]
fn test_format_datetime_mysql_format() {
    let dt: DateTime<Utc> = Utc.with_ymd_and_hms(2024, 6, 1, 8, 5, 3).unwrap();
    let s = format_datetime_for_driver(&dt, &Drivers::MySQL);
    // MySQL uses "YYYY-MM-DD HH:MM:SS.ffffff"
    assert!(s.starts_with("2024-06-01 08:05:03"));
    assert!(!s.contains('T'));
}

// ============================================================================
// format_datetime_fixed_for_driver
// ============================================================================

#[test]
fn test_format_datetime_fixed_postgres_keeps_offset() {
    let offset = FixedOffset::east_opt(2 * 3600).unwrap();
    let dt: DateTime<FixedOffset> = offset.with_ymd_and_hms(2024, 3, 10, 12, 0, 0).unwrap();
    let s = format_datetime_fixed_for_driver(&dt, &Drivers::Postgres);
    assert!(s.contains("+02:00"));
}

#[test]
fn test_format_datetime_fixed_mysql_converts_to_utc() {
    let offset = FixedOffset::east_opt(3 * 3600).unwrap();
    let dt: DateTime<FixedOffset> = offset.with_ymd_and_hms(2024, 3, 10, 15, 0, 0).unwrap();
    let s = format_datetime_fixed_for_driver(&dt, &Drivers::MySQL);
    // 15:00 +03:00 => 12:00 UTC
    assert!(s.starts_with("2024-03-10 12:00:00"));
}

#[test]
fn test_format_datetime_fixed_sqlite_is_rfc3339() {
    let offset = FixedOffset::east_opt(0).unwrap();
    let dt: DateTime<FixedOffset> = offset.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
    let s = format_datetime_fixed_for_driver(&dt, &Drivers::SQLite);
    assert!(s.contains('T'));
}

// ============================================================================
// format_naive_datetime_for_driver
// ============================================================================

#[test]
fn test_format_naive_datetime_postgres_mysql() {
    let dt = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2024, 11, 5).unwrap(),
        NaiveTime::from_hms_opt(7, 8, 9).unwrap(),
    );
    let pg = format_naive_datetime_for_driver(&dt, &Drivers::Postgres);
    let my = format_naive_datetime_for_driver(&dt, &Drivers::MySQL);
    assert!(pg.starts_with("2024-11-05 07:08:09"));
    assert_eq!(pg, my);
}

#[test]
fn test_format_naive_datetime_sqlite() {
    let dt = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2024, 11, 5).unwrap(),
        NaiveTime::from_hms_opt(7, 8, 9).unwrap(),
    );
    let s = format_naive_datetime_for_driver(&dt, &Drivers::SQLite);
    assert!(s.starts_with("2024-11-05 07:08:09"));
}

// ============================================================================
// get_postgres_type_cast
// ============================================================================

#[test]
fn test_postgres_type_cast_timestamptz() {
    assert_eq!(get_postgres_type_cast("TIMESTAMPTZ"), "::TIMESTAMPTZ");
    assert_eq!(get_postgres_type_cast("TIMESTAMP WITH TIME ZONE"), "::TIMESTAMPTZ");
    assert_eq!(get_postgres_type_cast("DateTime"), "::TIMESTAMPTZ");
}

#[test]
fn test_postgres_type_cast_timestamp() {
    assert_eq!(get_postgres_type_cast("TIMESTAMP"), "::TIMESTAMP");
    assert_eq!(get_postgres_type_cast("NaiveDateTime"), "::TIMESTAMP");
}

#[test]
fn test_postgres_type_cast_date() {
    assert_eq!(get_postgres_type_cast("DATE"), "::DATE");
    assert_eq!(get_postgres_type_cast("NaiveDate"), "::DATE");
}

#[test]
fn test_postgres_type_cast_time() {
    assert_eq!(get_postgres_type_cast("TIME"), "::TIME");
    assert_eq!(get_postgres_type_cast("NaiveTime"), "::TIME");
}

#[test]
fn test_postgres_type_cast_case_insensitive() {
    assert_eq!(get_postgres_type_cast("timestamptz"), "::TIMESTAMPTZ");
    assert_eq!(get_postgres_type_cast("date"), "::DATE");
}

#[test]
fn test_postgres_type_cast_unknown() {
    assert_eq!(get_postgres_type_cast("TEXT"), "");
    assert_eq!(get_postgres_type_cast("INTEGER"), "");
}

// ============================================================================
// is_temporal_type
// ============================================================================

#[test]
fn test_is_temporal_type_positive() {
    assert!(is_temporal_type("TIMESTAMPTZ"));
    assert!(is_temporal_type("TIMESTAMP WITH TIME ZONE"));
    assert!(is_temporal_type("TIMESTAMP"));
    assert!(is_temporal_type("TIMESTAMP WITHOUT TIME ZONE"));
    assert!(is_temporal_type("DateTime"));
    assert!(is_temporal_type("DATE"));
    assert!(is_temporal_type("TIME"));
    assert!(is_temporal_type("NaiveDateTime"));
    assert!(is_temporal_type("NaiveDate"));
    assert!(is_temporal_type("NaiveTime"));
}

#[test]
fn test_is_temporal_type_case_insensitive() {
    assert!(is_temporal_type("timestamptz"));
    assert!(is_temporal_type("date"));
    assert!(is_temporal_type("naivedatetime"));
}

#[test]
fn test_is_temporal_type_negative() {
    assert!(!is_temporal_type("TEXT"));
    assert!(!is_temporal_type("INTEGER"));
    assert!(!is_temporal_type("UUID"));
    assert!(!is_temporal_type("BOOLEAN"));
    assert!(!is_temporal_type(""));
}

// Bring chrono traits in scope for year/month/day/hour/etc
use chrono::{Datelike, Timelike};
