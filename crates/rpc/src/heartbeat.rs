use proto::heartbeat::{HeartbeatRequest, HeartbeatResponse};
use proto::status::Status;

pub struct HeartbeatService;

impl HeartbeatService {
    pub fn handle_heartbeat(&self, req: HeartbeatRequest) -> HeartbeatResponse {
        tracing::info!(
            "Received heartbeat from BE {}:{} (version: {})",
            req.be_host,
            req.be_port,
            req.be_version
        );
        HeartbeatResponse {
            status: Status::ok(),
            master_info: None,
        }
    }
}
