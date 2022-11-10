use std::fmt::{Display, Formatter};
use crate::entity::ReplayMode;

impl From<u8> for ReplayMode {
    fn from(v: u8) -> Self {
        match v {
            1 => ReplayMode::Cluster,
            2 => ReplayMode::Origin,
            3 => ReplayMode::Target,
            _ => ReplayMode::NA,
        }
    }
}

impl Display for ReplayMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayMode::Cluster => write!(f, "Cluster"),
            ReplayMode::Origin => write!(f, "Origin"),
            ReplayMode::Target => write!(f, "Target"),
            _ => write!(f, "NA"),
        }
    }
}

impl ReplayMode {
    pub fn value(&self) -> u8 {
        match self {
            ReplayMode::Cluster => 1,
            ReplayMode::Origin => 2,
            ReplayMode::Target => 3,
            ReplayMode::NA => 0,
        }
    }
}