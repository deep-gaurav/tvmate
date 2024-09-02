use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct HostParams {
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct JoinParams {
    pub name: String,
    pub room_id: String,
}
