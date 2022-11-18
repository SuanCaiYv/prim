use std::any::Any;
use std::sync::Arc;

use crate::core::mock::echo;
use ahash::AHashSet;
use common::net::server::GenericParameter;
use common::net::OuterSender;
use common::Result;
use dashmap::{DashMap, DashSet};
use lazy_static::lazy_static;

mod inner;
mod mock;
mod outer;

/// use Arc + ConcurrentMap + Clone to share state between Tasks
pub(self) struct ClientConnectionMap(Arc<DashMap<u64, OuterSender>>);
pub(self) struct UserStatusMap(Arc<DashMap<u64, u64>>);
pub(self) struct GroupUserList(Arc<DashMap<u64, AHashSet<u64>>>);
pub(self) struct GroupRecordedUserId(Arc<DashSet<u64>>);
/// map of node_id and node connection
pub(crate) type ClusterConnectionMap = Arc<DashMap<u32, OuterSender>>;

lazy_static! {
    static ref CONNECTION_MAP: ClientConnectionMap = ClientConnectionMap(Arc::new(DashMap::new()));
    static ref USER_STATUS_MAP: UserStatusMap = UserStatusMap(Arc::new(DashMap::new()));
    static ref GROUP_USER_LIST: GroupUserList = GroupUserList(Arc::new(DashMap::new()));
    static ref GROUP_RECORDED_USER_ID: GroupRecordedUserId =
        GroupRecordedUserId(Arc::new(DashSet::new()));
    static ref CLUSTER_CONNECTION_MAP: ClusterConnectionMap = Arc::new(DashMap::new());
}

impl GenericParameter for ClientConnectionMap {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl GenericParameter for UserStatusMap {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

pub(super) async fn start() -> Result<()> {
    outer::start().await?;
    inner::start().await?;
    Ok(())
}

#[allow(unused)]
pub(crate) async fn mock() -> Result<()> {
    echo(115, 916).await?;
    Ok(())
}

#[allow(unused)]
pub(self) fn get_client_connection_map() -> ClientConnectionMap {
    ClientConnectionMap(CONNECTION_MAP.0.clone())
}

#[allow(unused)]
pub(self) fn get_user_status_map() -> UserStatusMap {
    UserStatusMap(USER_STATUS_MAP.0.clone())
}

#[allow(unused)]
pub(self) fn get_group_user_list() -> GroupUserList {
    GroupUserList(GROUP_USER_LIST.0.clone())
}

#[allow(unused)]
pub(self) fn get_group_recorded_user_id() -> GroupRecordedUserId {
    GroupRecordedUserId(GROUP_RECORDED_USER_ID.0.clone())
}

#[allow(unused)]
pub(crate) fn get_cluster_connection_map() -> ClusterConnectionMap {
    CLUSTER_CONNECTION_MAP.clone()
}
