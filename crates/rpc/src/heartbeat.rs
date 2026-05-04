use proto::{HeartbeatRequest, HeartbeatResponse};

pub struct HeartbeatService;

impl HeartbeatService {
    pub fn handle_heartbeat(&self, req: HeartbeatRequest) -> HeartbeatResponse {
        tracing::info!(
            "Received heartbeat from BE (timestamp: {})",
            req.timestamp
        );
        HeartbeatResponse {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }
}