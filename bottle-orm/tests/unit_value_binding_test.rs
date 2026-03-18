use bottle_orm::{
    database::Drivers,
    value_binding::{bind_typed_value, is_numeric_type, is_text_type, requires_special_binding},
};
use sqlx::any::AnyArguments;

// ============================================================================
// Helpers
// ============================================================================

fn args() -> AnyArguments<'static> {
    AnyArguments::default()
}

// ============================================================================
// bind_value — successful parses
// ============================================================================

#[test]
fn test_bind_integer_valid() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "42", "INTEGER", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_integer_alias_int() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "0", "INT", &Drivers::Postgres).is_ok());
}

#[test]
fn test_bind_integer_u32_range() {
    // 2^31 + 1 exceeds i32 but fits u32 -> mapped to i64
    let mut a = args();
    assert!(bind_typed_value(&mut a, "2147483648", "INTEGER", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_bigint_valid() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "9223372036854775807", "BIGINT", &Drivers::Postgres).is_ok());
}

#[test]
fn test_bind_smallint_valid() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "32767", "SMALLINT", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_bool_true() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "true", "BOOLEAN", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_bool_false() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "false", "BOOL", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_float_double_precision() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "3.14159", "DOUBLE PRECISION", &Drivers::Postgres).is_ok());
}

#[test]
fn test_bind_float_numeric() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "99.99", "NUMERIC", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_real() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "1.5", "REAL", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_text() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "hello world", "TEXT", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_varchar() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "foo", "VARCHAR", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_unknown_type_falls_back_to_string() {
    let mut a = args();
    // Unknown types fall back to TEXT binding
    assert!(bind_typed_value(&mut a, "anything", "CUSTOM_TYPE", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_json_postgres() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, r#"{"key":"value"}"#, "JSON", &Drivers::Postgres).is_ok());
}

#[test]
fn test_bind_jsonb_sqlite() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, r#"[1,2,3]"#, "JSONB", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_uuid_valid() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "550e8400-e29b-41d4-a716-446655440000", "UUID", &Drivers::Postgres).is_ok());
}

#[test]
fn test_bind_uuid_mysql() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "550e8400-e29b-41d4-a716-446655440000", "UUID", &Drivers::MySQL).is_ok());
}

#[test]
fn test_bind_uuid_sqlite() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "550e8400-e29b-41d4-a716-446655440000", "UUID", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_timestamptz_utc() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "2024-01-15T14:30:00Z", "TIMESTAMPTZ", &Drivers::Postgres).is_ok());
}

#[test]
fn test_bind_datetime_alias() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "2024-01-15T14:30:00Z", "DateTime", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_timestamp_naive() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "2024-06-10 12:00:00", "TIMESTAMP", &Drivers::MySQL).is_ok());
}

#[test]
fn test_bind_naivedatetime_alias() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "2024-06-10 12:00:00", "NaiveDateTime", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_date() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "2024-03-15", "DATE", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_naivedate_alias() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "2024-03-15", "NaiveDate", &Drivers::Postgres).is_ok());
}

#[test]
fn test_bind_time() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "10:30:00", "TIME", &Drivers::SQLite).is_ok());
}

#[test]
fn test_bind_naivetime_alias() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "10:30:00", "NaiveTime", &Drivers::Postgres).is_ok());
}

#[test]
fn test_bind_array_postgres() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "{1,2,3}", "INTEGER[]", &Drivers::Postgres).is_ok());
}

#[test]
fn test_bind_array_sqlite_fallback() {
    let mut a = args();
    assert!(bind_typed_value(&mut a, "[1,2,3]", "TEXT[]", &Drivers::SQLite).is_ok());
}

// ============================================================================
// bind_value — error cases
// ============================================================================

#[test]
fn test_bind_integer_invalid() {
    let mut a = args();
    let result = bind_typed_value(&mut a, "abc", "INTEGER", &Drivers::SQLite);
    assert!(result.is_err());
}

#[test]
fn test_bind_bigint_invalid() {
    let mut a = args();
    let result = bind_typed_value(&mut a, "not_a_number", "BIGINT", &Drivers::Postgres);
    assert!(result.is_err());
}

#[test]
fn test_bind_smallint_overflow() {
    // 40000 exceeds i16::MAX (32767)
    let mut a = args();
    let result = bind_typed_value(&mut a, "40000", "SMALLINT", &Drivers::SQLite);
    assert!(result.is_err());
}

#[test]
fn test_bind_bool_invalid() {
    let mut a = args();
    let result = bind_typed_value(&mut a, "yes", "BOOLEAN", &Drivers::SQLite);
    assert!(result.is_err());
}

#[test]
fn test_bind_float_invalid() {
    let mut a = args();
    let result = bind_typed_value(&mut a, "not_float", "FLOAT", &Drivers::SQLite);
    assert!(result.is_err());
}

#[test]
fn test_bind_uuid_invalid() {
    let mut a = args();
    let result = bind_typed_value(&mut a, "not-a-uuid", "UUID", &Drivers::Postgres);
    assert!(result.is_err());
}

#[test]
fn test_bind_datetime_invalid() {
    let mut a = args();
    let result = bind_typed_value(&mut a, "not-a-date", "TIMESTAMPTZ", &Drivers::Postgres);
    assert!(result.is_err());
}

#[test]
fn test_bind_date_invalid() {
    let mut a = args();
    let result = bind_typed_value(&mut a, "32-13-2024", "DATE", &Drivers::SQLite);
    assert!(result.is_err());
}

#[test]
fn test_bind_time_invalid() {
    let mut a = args();
    let result = bind_typed_value(&mut a, "99:99:99", "TIME", &Drivers::SQLite);
    assert!(result.is_err());
}

// ============================================================================
// requires_special_binding
// ============================================================================

#[test]
fn test_requires_special_binding_temporal() {
    assert!(requires_special_binding("TIMESTAMPTZ"));
    assert!(requires_special_binding("DateTime"));
    assert!(requires_special_binding("TIMESTAMP"));
    assert!(requires_special_binding("NaiveDateTime"));
    assert!(requires_special_binding("DATE"));
    assert!(requires_special_binding("NaiveDate"));
    assert!(requires_special_binding("TIME"));
    assert!(requires_special_binding("NaiveTime"));
}

#[test]
fn test_requires_special_binding_uuid() {
    assert!(requires_special_binding("UUID"));
}

#[test]
fn test_requires_special_binding_false() {
    assert!(!requires_special_binding("TEXT"));
    assert!(!requires_special_binding("INTEGER"));
    assert!(!requires_special_binding("BOOLEAN"));
    assert!(!requires_special_binding("JSON"));
}

// ============================================================================
// is_numeric_type
// ============================================================================

#[test]
fn test_is_numeric_type_positive() {
    assert!(is_numeric_type("INTEGER"));
    assert!(is_numeric_type("INT"));
    assert!(is_numeric_type("BIGINT"));
    assert!(is_numeric_type("INT8"));
    assert!(is_numeric_type("SERIAL"));
    assert!(is_numeric_type("BIGSERIAL"));
    assert!(is_numeric_type("SMALLINT"));
    assert!(is_numeric_type("DOUBLE PRECISION"));
    assert!(is_numeric_type("FLOAT"));
    assert!(is_numeric_type("REAL"));
    assert!(is_numeric_type("NUMERIC"));
    assert!(is_numeric_type("DECIMAL"));
}

#[test]
fn test_is_numeric_type_negative() {
    assert!(!is_numeric_type("TEXT"));
    assert!(!is_numeric_type("UUID"));
    assert!(!is_numeric_type("BOOLEAN"));
    assert!(!is_numeric_type("TIMESTAMPTZ"));
}

// ============================================================================
// is_text_type
// ============================================================================

#[test]
fn test_is_text_type_positive() {
    assert!(is_text_type("TEXT"));
    assert!(is_text_type("VARCHAR"));
    assert!(is_text_type("CHAR"));
    assert!(is_text_type("STRING"));
}

#[test]
fn test_is_text_type_negative() {
    assert!(!is_text_type("INTEGER"));
    assert!(!is_text_type("UUID"));
    assert!(!is_text_type("JSON"));
    assert!(!is_text_type("BOOLEAN"));
}
