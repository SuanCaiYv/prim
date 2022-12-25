use salvo::{handler, Request, Response};

use super::HandlerResult;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UploadReq {
    file_content: Vec<u8>,
    file_name: Option<String>,
    file_size: Option<u64>,
    file_id: String,
}

#[handler]
pub(crate) async fn upload(_req: &mut Request, _res: &mut Response) -> HandlerResult {
    Ok(())
}

#[handler]
pub(crate) async fn download(_req: &mut Request, _res: &mut Response) -> HandlerResult {
    Ok(())
}