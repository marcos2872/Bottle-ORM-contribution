use bottle_orm::Error;

// ============================================================================
// Error variant construction
// ============================================================================

#[test]
fn test_invalid_data_direct() {
    let err = Error::InvalidData("bad input".to_string());
    assert!(matches!(err, Error::InvalidData(_)));
}

#[test]
fn test_conversion_direct() {
    let err = Error::Conversion("parse failed".to_string());
    assert!(matches!(err, Error::Conversion(_)));
}

#[test]
fn test_invalid_argument_direct() {
    let err = Error::InvalidArgument("negative limit".to_string());
    assert!(matches!(err, Error::InvalidArgument(_)));
}

// ============================================================================
// Convenience constructors
// ============================================================================

#[test]
fn test_invalid_data_constructor() {
    let err = Error::invalid_data("something went wrong");
    match err {
        Error::InvalidData(msg) => assert_eq!(msg, "something went wrong"),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn test_invalid_argument_constructor() {
    let err = Error::invalid_argument("page must be >= 0");
    match err {
        Error::InvalidArgument(msg) => assert_eq!(msg, "page must be >= 0"),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn test_conversion_constructor() {
    let err = Error::conversion("failed to parse UUID");
    match err {
        Error::Conversion(msg) => assert_eq!(msg, "failed to parse UUID"),
        _ => panic!("wrong variant"),
    }
}

// ============================================================================
// Display formatting
// ============================================================================

#[test]
fn test_invalid_data_display() {
    let err = Error::InvalidData("oops".to_string());
    assert_eq!(format!("{}", err), "Invalid Data: oops");
}

#[test]
fn test_conversion_display() {
    let err = Error::Conversion("bad date".to_string());
    assert_eq!(format!("{}", err), "Type conversion error: bad date");
}

#[test]
fn test_invalid_argument_display() {
    let err = Error::InvalidArgument("limit < 0".to_string());
    assert_eq!(format!("{}", err), "Invalid argument: limit < 0");
}

#[test]
fn test_database_error_display() {
    let sqlx_err = sqlx::Error::RowNotFound;
    let err = Error::DatabaseError(sqlx_err);
    let display = format!("{}", err);
    assert!(display.contains("Database error"));
}

// ============================================================================
// From<sqlx::Error> conversion
// ============================================================================

#[test]
fn test_from_sqlx_error() {
    let sqlx_err = sqlx::Error::RowNotFound;
    let err: Error = sqlx_err.into();
    assert!(matches!(err, Error::DatabaseError(_)));
}

// ============================================================================
// std::error::Error implementation
// ============================================================================

#[test]
fn test_error_implements_std_error() {
    let err: Box<dyn std::error::Error> = Box::new(Error::invalid_data("test"));
    assert!(err.to_string().contains("test"));
}
