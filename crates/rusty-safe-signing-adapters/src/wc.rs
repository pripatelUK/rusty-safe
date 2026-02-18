use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use rusty_safe_signing_core::{
    PendingWalletConnectRequest, PortError, WalletConnectPort, WcSessionAction, WcSessionContext,
    WcSessionStatus,
};

#[derive(Debug, Clone, Default)]
pub struct WalletConnectAdapter {
    inner: Arc<Mutex<WalletConnectState>>,
}

#[derive(Debug, Default)]
struct WalletConnectState {
    sessions: HashMap<String, WcSessionContext>,
    requests: HashMap<String, PendingWalletConnectRequest>,
}

impl WalletConnectAdapter {
    pub fn insert_session(&self, session: WcSessionContext) -> Result<(), PortError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("wc lock poisoned: {e}")))?;
        g.sessions.insert(session.topic.clone(), session);
        Ok(())
    }

    pub fn insert_request(&self, req: PendingWalletConnectRequest) -> Result<(), PortError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("wc lock poisoned: {e}")))?;
        g.requests.insert(req.request_id.clone(), req);
        Ok(())
    }
}

impl WalletConnectPort for WalletConnectAdapter {
    fn session_action(&self, topic: &str, action: WcSessionAction) -> Result<(), PortError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("wc lock poisoned: {e}")))?;
        let session = g
            .sessions
            .get_mut(topic)
            .ok_or_else(|| PortError::NotFound(format!("wc session missing: {topic}")))?;
        session.status = match action {
            WcSessionAction::Approve => WcSessionStatus::Approved,
            WcSessionAction::Reject => WcSessionStatus::Rejected,
            WcSessionAction::Disconnect => WcSessionStatus::Disconnected,
        };
        Ok(())
    }

    fn list_sessions(&self) -> Result<Vec<WcSessionContext>, PortError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("wc lock poisoned: {e}")))?;
        Ok(g.sessions.values().cloned().collect())
    }

    fn list_pending_requests(&self) -> Result<Vec<PendingWalletConnectRequest>, PortError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("wc lock poisoned: {e}")))?;
        Ok(g.requests.values().cloned().collect())
    }

    fn respond_success(&self, request_id: &str, _result: Value) -> Result<(), PortError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("wc lock poisoned: {e}")))?;
        if g.requests.remove(request_id).is_none() {
            return Err(PortError::NotFound(format!(
                "wc request missing for success response: {request_id}"
            )));
        }
        Ok(())
    }

    fn respond_error(&self, request_id: &str, _code: i64, _message: &str) -> Result<(), PortError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| PortError::Transport(format!("wc lock poisoned: {e}")))?;
        if g.requests.remove(request_id).is_none() {
            return Err(PortError::NotFound(format!(
                "wc request missing for error response: {request_id}"
            )));
        }
        Ok(())
    }
}
