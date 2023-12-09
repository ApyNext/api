use axum::response::{IntoResponse, Response};
use hyper::StatusCode;

#[derive(Debug)]
pub struct AppError {
    status_code: StatusCode,
    message: Option<String>,
}

impl AppError {
    pub fn new(status_code: StatusCode, message: Option<impl Into<String>>) -> Self {
        if let Some(message) = message {
            AppError {
                status_code,
                message: Some(message.into()),
            }
        } else {
            AppError {
                status_code,
                message: None,
            }
        }
    }
    pub fn internal_server_error() -> Self {
        AppError {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            message: None,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if let Some(message) = self.message {
            (self.status_code, message).into_response()
        } else {
            self.status_code.into_response()
        }
    }
}
