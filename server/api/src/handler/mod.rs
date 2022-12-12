use chrono::{DateTime, Local};
use salvo::{writer::Json, Piece, Response};

pub(crate) mod group;
pub(crate) mod msg;
pub(crate) mod relationship;
pub(crate) mod user;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(self) struct ResponseResult<'a, T>
where
    T: Send + Sync + 'static,
{
    code: u32,
    message: &'a str,
    timestamp: DateTime<Local>,
    data: T,
}

impl<'a, T: Send + Sync + 'static + serde::Serialize> Piece for ResponseResult<'a, T> {
    fn render(self, res: &mut Response) {
        res.render(Json(self));
    }
}
