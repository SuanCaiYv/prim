use thiserror::Error;

#[allow(unused)]
#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("should be passed down stream.")]
    NotMine,
    #[error("auth error: `{0}`")]
    Auth(String),
    #[error("parse msg error: `{0}`")]
    Parse(String),
}

#[allow(unused)]
#[derive(Debug, Clone, Error)]
pub enum MessageError {
    #[error("read msg head error: `{0}`")]
    ReadHeadError(String),
    #[error("read msg body error: `{0}`")]
    ReadBodyError(String),
    #[error("read msg timeout")]
    ReadTimeout,
}

#[allow(unused)]
#[derive(Debug, Error)]
pub enum CrashError {
    #[error("crash error: `{0}`")]
    ShouldCrash(String),
}
