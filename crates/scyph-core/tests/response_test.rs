use scyph_core::PagedResponse;

#[test]
fn test_paged_response_overflow_safety() {
    let paged: PagedResponse<i32> = PagedResponse::new(vec![], 100, i64::MAX, i64::MAX);
    assert!(!paged.has_next);
}
