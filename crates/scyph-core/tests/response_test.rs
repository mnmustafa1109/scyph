use axum::http::StatusCode;
use axum::response::IntoResponse;
use scyph_core::{ApiResponse, AppError, ErrorDetails, PagedResponse, ResponseMeta};

#[test]
fn test_paged_response_overflow_safety() {
    let paged: PagedResponse<i32> = PagedResponse::new(vec![], 100, i64::MAX, i64::MAX);
    assert!(!paged.has_next);
}

#[test]
fn test_api_response_constructors_and_status_codes() {
    let ok = ApiResponse::ok("data_1");
    assert_eq!(ok.status, StatusCode::OK);
    assert_eq!(ok.message, "OK");
    assert_eq!(ok.data, Some("data_1"));
    assert_eq!(ok.into_response().status(), StatusCode::OK);

    let ok_msg = ApiResponse::ok_msg("data_2", "Custom OK");
    assert_eq!(ok_msg.status, StatusCode::OK);
    assert_eq!(ok_msg.message, "Custom OK");
    assert_eq!(ok_msg.into_response().status(), StatusCode::OK);

    let created = ApiResponse::created("data_3");
    assert_eq!(created.status, StatusCode::CREATED);
    assert_eq!(created.message, "Created");
    assert_eq!(created.data, Some("data_3"));
    assert_eq!(created.into_response().status(), StatusCode::CREATED);

    let created_msg = ApiResponse::created_msg("data_4", "Resource Created");
    assert_eq!(created_msg.status, StatusCode::CREATED);
    assert_eq!(created_msg.message, "Resource Created");
    assert_eq!(created_msg.into_response().status(), StatusCode::CREATED);

    let accepted = ApiResponse::accepted("data_5");
    assert_eq!(accepted.status, StatusCode::ACCEPTED);
    assert_eq!(accepted.message, "Accepted");
    assert_eq!(accepted.into_response().status(), StatusCode::ACCEPTED);

    let custom = ApiResponse::ok("data_6").with_status(StatusCode::NON_AUTHORITATIVE_INFORMATION);
    assert_eq!(custom.status(), StatusCode::NON_AUTHORITATIVE_INFORMATION);
    assert_eq!(
        custom.into_response().status(),
        StatusCode::NON_AUTHORITATIVE_INFORMATION
    );

    let no_content = ApiResponse::no_content();
    assert_eq!(no_content.status, StatusCode::NO_CONTENT);
    assert_eq!(no_content.data, None);
    assert_eq!(no_content.into_response().status(), StatusCode::NO_CONTENT);
}

#[test]
fn test_api_response_serialization_excludes_status_field() {
    let res = ApiResponse::created("test_data");
    let json_str = serde_json::to_string(&res).expect("serialization succeeds");

    // The wire JSON must not contain "status" (which is reserved for HTTP status in RFC 7807/REST)
    let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid JSON");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["message"], "Created");
    assert_eq!(parsed["data"], "test_data");
    assert!(parsed.get("status").is_none());
    assert!(parsed.get("meta").is_none());
}

#[test]
fn test_api_response_with_meta() {
    let meta = ResponseMeta::new("trace-1234", 12, "0.1.0");
    let res = ApiResponse::ok("data").with_meta(meta.clone());
    assert_eq!(res.meta, Some(meta));

    let json_str = serde_json::to_string(&res).expect("serialization succeeds");
    let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("valid JSON");
    assert_eq!(parsed["meta"]["trace_id"], "trace-1234");
    assert_eq!(parsed["meta"]["processing_time_ms"], 12);
}

#[test]
fn test_app_error_validation_response_generation() {
    let details = vec![ErrorDetails {
        field: Some("email".into()),
        code: "invalid_format".into(),
        message: "Must be a valid email address".into(),
    }];

    let err = AppError::ValidationError {
        message: "Validation failed".into(),
        details,
    };

    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
