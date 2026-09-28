use scyph_core::PagedResponse;

#[test]
fn test_paged_response_overflow_safety() {
    let paged: PagedResponse<i32> = PagedResponse::new(vec![], 100, i64::MAX, i64::MAX);
    assert!(!paged.has_next);
}

#[test]
fn test_api_response_created_with_meta() {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use scyph_core::{ApiResponse, ResponseMeta};

    let meta = ResponseMeta::new("trace-123", 5, "0.1.0");
    let resp = ApiResponse::created("entity_id").with_meta(meta);

    assert_eq!(resp.status(), StatusCode::CREATED);
    assert_eq!(resp.message, "Created");
    assert!(resp.meta.is_some());
    assert_eq!(resp.data, Some("entity_id"));

    let http_resp = resp.into_response();
    assert_eq!(http_resp.status(), StatusCode::CREATED);
}
