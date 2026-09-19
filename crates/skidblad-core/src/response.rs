// crates/skidblad-core/src/response.rs
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// Standard JSON envelope for successful API responses.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    /// 200 OK with data
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            message: "OK".into(),
            data: Some(data),
        }
    }

    /// 200 OK with data and a custom message
    pub fn ok_msg(data: T, msg: impl Into<String>) -> Self {
        Self {
            success: true,
            message: msg.into(),
            data: Some(data),
        }
    }

    /// 201 Created
    pub fn created(data: T) -> impl IntoResponse {
        (
            StatusCode::CREATED,
            Json(Self {
                success: true,
                message: "Created".into(),
                data: Some(data),
            }),
        )
    }
}

impl ApiResponse<()> {
    /// 204 No Content
    pub fn no_content() -> impl IntoResponse {
        StatusCode::NO_CONTENT
    }
}

impl<T: Serialize + Send> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

/// Cursor-based pagination envelope.
#[derive(Debug, Serialize)]
pub struct PagedResponse<T: Serialize> {
    pub success: bool,
    pub message: String,
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub has_next: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl<T: Serialize + Send> IntoResponse for PagedResponse<T> {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
