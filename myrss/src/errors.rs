use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub enum ApiError {
    HTTPError(axum::http::Error),
    #[allow(dead_code)]
    DoesNotExist,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::HTTPError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("HTTP error: {e}"),
            )
                .into_response(),
            Self::DoesNotExist => StatusCode::NOT_FOUND.into_response(),
        }
    }
}

impl From<axum::http::Error> for ApiError {
    fn from(e: axum::http::Error) -> Self {
        Self::HTTPError(e)
    }
}
