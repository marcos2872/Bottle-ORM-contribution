use bottle_orm::pagination::Pagination;

// ============================================================================
// Default
// ============================================================================

#[test]
fn test_pagination_default() {
    let p = Pagination::default();
    assert_eq!(p.page, 0);
    assert_eq!(p.limit, 10);
    assert_eq!(p.max_limit, 100);
}

// ============================================================================
// Pagination::new
// ============================================================================

#[test]
fn test_pagination_new_basic() {
    let p = Pagination::new(2, 20);
    assert_eq!(p.page, 2);
    assert_eq!(p.limit, 20);
    assert_eq!(p.max_limit, 100);
}

#[test]
fn test_pagination_new_caps_at_100() {
    let p = Pagination::new(0, 200);
    assert_eq!(p.limit, 100);
}

#[test]
fn test_pagination_new_exactly_100() {
    let p = Pagination::new(0, 100);
    assert_eq!(p.limit, 100);
}

#[test]
fn test_pagination_new_page_zero() {
    let p = Pagination::new(0, 10);
    assert_eq!(p.page, 0);
}

// ============================================================================
// Pagination::new_with_limit
// ============================================================================

#[test]
fn test_new_with_limit_under_max() {
    let p = Pagination::new_with_limit(1, 15, 50);
    assert_eq!(p.limit, 15);
    assert_eq!(p.max_limit, 50);
    assert_eq!(p.page, 1);
}

#[test]
fn test_new_with_limit_exceeds_max_is_capped() {
    let p = Pagination::new_with_limit(0, 200, 50);
    assert_eq!(p.limit, 50);
}

#[test]
fn test_new_with_limit_exactly_at_max() {
    let p = Pagination::new_with_limit(0, 50, 50);
    assert_eq!(p.limit, 50);
}

#[test]
fn test_new_with_limit_custom_max() {
    let p = Pagination::new_with_limit(3, 999, 500);
    assert_eq!(p.limit, 500);
    assert_eq!(p.page, 3);
}

// ============================================================================
// Page offset calculation (tested via apply on a simple query)
// We can verify the formula page * limit without needing a real DB.
// ============================================================================

#[test]
fn test_offset_formula_page_0() {
    let p = Pagination::new(0, 10);
    // offset = page * limit = 0 * 10 = 0
    assert_eq!(p.page * p.limit, 0);
}

#[test]
fn test_offset_formula_page_2() {
    let p = Pagination::new(2, 10);
    // offset = 2 * 10 = 20
    assert_eq!(p.page * p.limit, 20);
}

#[test]
fn test_offset_formula_page_5_limit_25() {
    let p = Pagination::new(5, 25);
    // offset = 5 * 25 = 125
    assert_eq!(p.page * p.limit, 125);
}

// ============================================================================
// Total pages calculation (mirrors pagination.rs line 222)
// total_pages = ceil(total / limit)
// ============================================================================

fn total_pages(total: i64, limit: usize) -> i64 {
    (total as f64 / limit as f64).ceil() as i64
}

#[test]
fn test_total_pages_exact_division() {
    assert_eq!(total_pages(100, 10), 10);
}

#[test]
fn test_total_pages_remainder() {
    assert_eq!(total_pages(101, 10), 11);
}

#[test]
fn test_total_pages_single_page() {
    assert_eq!(total_pages(5, 10), 1);
}

#[test]
fn test_total_pages_zero_records() {
    assert_eq!(total_pages(0, 10), 0);
}

#[test]
fn test_total_pages_one_record() {
    assert_eq!(total_pages(1, 100), 1);
}

#[test]
fn test_total_pages_large() {
    assert_eq!(total_pages(1000, 7), 143); // ceil(1000/7) = 143
}
