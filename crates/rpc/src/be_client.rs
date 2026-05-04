use proto::BeServiceClient;
use proto::{ExecPlanFragmentRequest, ExecPlanFragmentResponse, CancelPlanFragmentRequest, FetchDataRequest, FetchDataResponse, HeartbeatRequest, HeartbeatResponse, Status};
use tonic::transport::Channel;

/// gRPC client for connecting to Backend nodes
pub struct BeGrpcClient {
    client: BeServiceClient<Channel>,
    addr: String,
}

impl BeGrpcClient {
    /// Create a new BE gRPC client
    pub async fn connect(addr: String) -> Result<Self, tonic::transport::Error> {
        let client = BeServiceClient::connect(format!("http://{}", addr)).await?;
        Ok(Self { client, addr })
    }

    /// Execute a plan fragment on the backend
    pub async fn exec_plan_fragment(
        &self,
        req: ExecPlanFragmentRequest,
    ) -> Result<ExecPlanFragmentResponse, tonic::Status> {
        let mut client = self.client.clone();
        let response = client.exec_plan_fragment(req).await?;
        Ok(response.into_inner())
    }

    /// Cancel a running plan fragment
    pub async fn cancel_plan_fragment(
        &self,
        fragment_instance_id: String,
    ) -> Result<Status, tonic::Status> {
        let mut client = self.client.clone();
        let req = CancelPlanFragmentRequest { fragment_id: fragment_instance_id };
        let response = client.cancel_plan_fragment(req).await?;
        Ok(response.into_inner())
    }

    /// Fetch data from a completed fragment
    pub async fn fetch_data(
        &self,
        fragment_instance_id: String,
        max_rows: i32,
    ) -> Result<FetchDataResponse, tonic::Status> {
        let mut client = self.client.clone();
        let req = FetchDataRequest {
            fragment_id: fragment_instance_id,
            max_rows,
        };
        let response = client.fetch_data(req).await?;
        Ok(response.into_inner())
    }

    /// Send heartbeat to backend
    pub async fn heartbeat(
        &self,
    ) -> Result<HeartbeatResponse, tonic::Status> {
        let mut client = self.client.clone();
        let req = HeartbeatRequest {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };
        let response: tonic::Response<HeartbeatResponse> = client.heartbeat(req).await?;
        Ok(response.into_inner())
    }

    /// Get the backend address
    pub fn addr(&self) -> &str {
        &self.addr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires actual BE server
    async fn test_be_client_connection() {
        let client = BeGrpcClient::connect("localhost:50051".to_string()).await;
        assert!(client.is_ok());
    }
}
