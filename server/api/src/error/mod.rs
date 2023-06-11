use async_trait::async_trait;
use chrono::Local;
use salvo::{Depot, Request, Response, Writer, prelude::StatusCode};

use crate::handler::ResponseResult;

pub(crate) enum HandlerError {
    /// used for leak of request parameters.
    ParameterMismatch(String),
    /// used for invalid request parameters.
    RequestMismatch(u32, String),
    /// used for server internal errors.
    InternalError(String),
}

#[async_trait]
impl Writer for HandlerError {
    async fn write(self, _req: &mut Request, _depot: &mut Depot, resp: &mut Response) {
        match self {
            HandlerError::ParameterMismatch(msg) => {
                resp.set_status_code(StatusCode::OK);
                resp.render(
                    ResponseResult {
                        code: 400,
                        message: &msg,
                        timestamp: Local::now(),
                        data: (),
                    }
                )
            }
            HandlerError::RequestMismatch(code, msg) => {
                resp.set_status_code(StatusCode::OK);
                resp.render(
                    ResponseResult {
                        code,
                        message: &msg,
                        timestamp: Local::now(),
                        data: (),
                    }
                )
            }
            HandlerError::InternalError(msg) => {
                resp.set_status_code(StatusCode::OK);
                resp.render(
                    ResponseResult {
                        code: 500,
                        message: &msg,
                        timestamp: Local::now(),
                        data: (),
                    }
                )
            }
        }
    }
}
