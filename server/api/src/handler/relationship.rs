use salvo::handler;

#[handler]
pub(crate) async fn add_friend(_req: &mut salvo::Request, _res: &mut salvo::Response) {
    todo!()
}

#[handler]
pub(crate) async fn approve_add_friend(_req: &mut salvo::Request, _res: &mut salvo::Response) {
    todo!()
}

/// only both sides all invoke this method, the relationship will be dropped.
#[handler]
pub(crate) async fn delete_friend(_req: &mut salvo::Request, _res: &mut salvo::Response) {
    todo!()
}

#[handler]
pub(crate) async fn get_friend_list(_req: &mut salvo::Request, _res: &mut salvo::Response) {
    todo!()
}

/// 1. not friend
/// 2. friend but with different status, such as: normal, best friend, block, lover...
#[handler]
pub(crate) async fn get_peer_relationship(_req: &mut salvo::Request, _res: &mut salvo::Response) {
    todo!()
}

/// only work on user and peer is already friend.
#[handler]
pub(crate) async fn update_relationship(_req: &mut salvo::Request, _res: &mut salvo::Response) {
    todo!()
}