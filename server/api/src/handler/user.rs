use crate::entity::user::User;
use crate::rpc::get_rpc_client;
use chrono::{DateTime, Local};
use salvo::prelude::Json;
use salvo::{handler, Piece, Request, Response};
use tracing::error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ResponseResult<T>
where
    T: Send + Sync + 'static,
{
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
pub(crate) async fn new_account_id(_: &mut Request, resp: &mut Response) {
    // todo optimization
    loop {
        // todo threshold range
        let id: u64 = fastrand::u64((1 << 33) + 1..1 << 34);
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
pub(crate) async fn login(_req: &mut Request, _resp: &mut Response) {}

#[handler]
pub(crate) async fn logout(_req: &mut Request, _resp: &mut Response) {}

#[handler]
pub(crate) async fn signup(_req: &mut Request, _resp: &mut Response) {}

#[handler]
pub(crate) async fn sign_out(_req: &mut Request, _resp: &mut Response) {}

#[handler]
pub(crate) async fn which_node(req: &mut Request, resp: &mut Response) {
    let user_id = req.param::<u64>("user_id");
    if user_id.is_none() {
        resp.render(ResponseResult {
            code: 400,
            message: "user_id is required".to_string(),
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let user_id = user_id.unwrap();
    let res = get_rpc_client().await.call_which_node(user_id).await;
    if res.is_err() {
        error!("which_node error: {}", res.err().unwrap().to_string());
        resp.render(ResponseResult {
            code: 500,
            message: "server error".to_string(),
            timestamp: Local::now(),
            data: (),
        });
        return;
    }
    let res = res.unwrap();
    resp.render(ResponseResult {
        code: 200,
        message: "ok".to_string(),
        timestamp: Local::now(),
        data: res,
    });
}
