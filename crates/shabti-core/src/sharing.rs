use std::collections::HashMap;

use uuid::Uuid;

use crate::error::{ShabtiError, ShabtiResult};
use crate::namespace::NamespacePolicy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareStatus {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareResponse {
    Accepted,
    Rejected,
}

#[derive(Debug, Clone)]
pub struct ShareRequest {
    pub id: Uuid,
    pub from_agent: Uuid,
    pub to_agent: Uuid,
    pub namespace: String,
    pub reason: String,
    pub status: ShareStatus,
}

impl ShareRequest {
    pub fn new(from_agent: Uuid, to_agent: Uuid, namespace: &str, reason: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_agent,
            to_agent,
            namespace: namespace.to_string(),
            reason: reason.to_string(),
            status: ShareStatus::Pending,
        }
    }

    pub fn is_pending(&self) -> bool {
        self.status == ShareStatus::Pending
    }
}

#[derive(Debug, Default)]
pub struct SharingProtocol {
    requests: HashMap<Uuid, ShareRequest>,
}

impl SharingProtocol {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request_share(
        &mut self,
        from_agent: Uuid,
        to_agent: Uuid,
        namespace: &str,
        reason: &str,
    ) -> Uuid {
        let request = ShareRequest::new(from_agent, to_agent, namespace, reason);
        let id = request.id;
        self.requests.insert(id, request);
        id
    }

    pub fn respond(
        &mut self,
        request_id: Uuid,
        response: ShareResponse,
        policy: &mut NamespacePolicy,
    ) -> ShabtiResult<()> {
        let request = self
            .requests
            .get_mut(&request_id)
            .ok_or_else(|| ShabtiError::Storage(format!("request {request_id} not found")))?;

        if !request.is_pending() {
            return Err(ShabtiError::Storage(format!(
                "request {request_id} already resolved"
            )));
        }

        match response {
            ShareResponse::Accepted => {
                request.status = ShareStatus::Accepted;
                policy.grant_access(request.to_agent, &request.namespace);
            }
            ShareResponse::Rejected => {
                request.status = ShareStatus::Rejected;
            }
        }

        Ok(())
    }

    pub fn pending_for(&self, agent_id: Uuid) -> Vec<&ShareRequest> {
        self.requests
            .values()
            .filter(|r| r.to_agent == agent_id && r.is_pending())
            .collect()
    }
}
