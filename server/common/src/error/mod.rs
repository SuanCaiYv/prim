use thiserror::Error;

#[allow(unused)]
#[derive(Debug, Error)]
pub(crate) enum HandlerError {
    #[error("should be passed down stream.")]
    NotMine,
    #[error("auth error: `{0}`")]
    Auth(String),
}

#[allow(unused)]
#[derive(Debug, Clone, Error)]
pub(crate) enum MessageError {
    #[error("read msg head error: `{0}`")]
    ReadHeadError(String),
    #[error("read msg body error: `{0}`")]
    ReadBodyError(String),
}

#[allow(unused)]
#[derive(Debug, Error)]
pub(crate) enum CrashError {
    #[error("crash error: `{0}`")]
    ShouldCrash(String),
}
