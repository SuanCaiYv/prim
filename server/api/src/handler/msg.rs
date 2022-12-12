use salvo::handler;

#[handler]
pub(crate) async fn inbox(_req: &mut salvo::Request, _res: &mut salvo::Response) {
    todo!()
}

#[handler]
pub(crate) async fn msg_history_record(_req: &mut salvo::Request, _res: &mut salvo::Response) {
    todo!()
}

#[handler]
pub(crate) async fn withdraw(_req: &mut salvo::Request, _res: &mut salvo::Response) {
    todo!()
}

#[handler]
pub(crate) async fn edit(_req: &mut salvo::Request, _res: &mut salvo::Response) {
    todo!()
}