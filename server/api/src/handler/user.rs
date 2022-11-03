use chrono::{DateTime, Local};
use rand::{thread_rng, Rng};
use salvo::{handler, Piece, Request, Response};
use salvo::prelude::Json;
use crate::entity::user::User;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ResponseResult<T> where T: Send + Sync + 'static {
    code: u32,
    message: String,
    timestamp: DateTime<Local>,
    data: T,
}

impl<T: Send + Sync + 'static + serde::Serialize> Piece for ResponseResult<T> {
    fn render(self, res: &mut Response) {
        res.render(Json(self));
    }
}

#[handler]
pub(crate) async fn new_account_id(req: &mut Request, resp: &mut Response) {
    let mut rng = thread_rng();
    // todo optimization
    loop {
        let id: u64 = rng.gen_range(1 << 33 + 1, 1 << 62);
        let res = User::get_account_id(id as i64).await;
        if res.is_err() {
            resp.render(ResponseResult {
                code: 200,
                message: "ok".to_string(),
                timestamp: Local::now(),
                data: id,
            });
            break;
        }
    }
}

#[handler]
pub(crate) async fn login(req: &mut Request, resp: &mut Response) {}

#[handler]
pub(crate) async fn logout(req: &mut Request, resp: &mut Response) {}

#[handler]
pub(crate) async fn signup(req: &mut Request, resp: &mut Response) {}

#[handler]
pub(crate) async fn sign_out(req: &mut Request, resp: &mut Response) {}

#[handler]
pub(crate) async fn which_node(req: &mut Request, resp: &mut Response) {}
