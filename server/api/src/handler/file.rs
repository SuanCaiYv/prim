use salvo::{handler, Request, Response};

use super::{HandlerResult, ResponseResult};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UploadReq {
    file_content: Vec<u8>,
    file_name: Option<String>,
    file_size: Option<u64>,
    file_id: String,
}

#[handler]
pub(crate) async fn upload(_req: &mut Request, _res: &mut Response) -> HandlerResult<'static, ()> {
    Ok(ResponseResult {
        code: 200,
        message: "success",
        timestamp: chrono::Local::now(),
        data: (),
    })
}

#[handler]
pub(crate) async fn download(
    _req: &mut Request,
    _res: &mut Response,
) -> HandlerResult<'static, ()> {
    Ok(ResponseResult {
        code: 200,
        message: "success",
        timestamp: chrono::Local::now(),
        data: (),
    })
}
