#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningSurface {
    Queue,
    TxDetails,
    MessageDetails,
    WalletConnect,
    ImportExport,
}

#[derive(Debug, Clone)]
pub struct SigningUiState {
    pub active_surface: SigningSurface,
    pub selected_flow_id: Option<String>,
    pub active_chain_id: u64,
    pub active_safe_address: String,
    pub queue_rows: Vec<SigningQueueRow>,
    pub tx_form: TxFormState,
    pub message_form: MessageFormState,
    pub wc_state: WalletConnectSurfaceState,
    pub import_export_state: ImportExportSurfaceState,
    pub last_error: Option<String>,
    pub last_info: Option<String>,
}

impl Default for SigningUiState {
    fn default() -> Self {
        Self {
            active_surface: SigningSurface::Queue,
            selected_flow_id: None,
            active_chain_id: 1,
            active_safe_address: String::new(),
            queue_rows: Vec::new(),
            tx_form: TxFormState::default(),
            message_form: MessageFormState::default(),
            wc_state: WalletConnectSurfaceState::default(),
            import_export_state: ImportExportSurfaceState::default(),
            last_error: None,
            last_info: None,
        }
    }
}

impl SigningUiState {
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.last_info = None;
        self.last_error = Some(message.into());
    }

    pub fn set_info(&mut self, message: impl Into<String>) {
        self.last_error = None;
        self.last_info = Some(message.into());
    }

    pub fn clear_notice(&mut self) {
        self.last_error = None;
        self.last_info = None;
    }
}

#[derive(Debug, Clone)]
pub struct SigningQueueRow {
    pub flow_id: String,
    pub flow_kind: String,
    pub state: String,
    pub signature_progress: String,
    pub origin: String,
    pub build_source: Option<String>,
    pub updated_ms: u64,
}

#[derive(Debug, Clone)]
pub struct TxFormState {
    pub nonce: String,
    pub to: String,
    pub value: String,
    pub data: String,
    pub threshold: String,
    pub safe_version: String,
    pub abi_json: String,
    pub abi_method: String,
    pub abi_args: String,
    pub manual_signer: String,
    pub manual_signature: String,
    pub confirm_signature: String,
}

impl Default for TxFormState {
    fn default() -> Self {
        Self {
            nonce: "0".to_owned(),
            to: String::new(),
            value: "0".to_owned(),
            data: "0x".to_owned(),
            threshold: "1".to_owned(),
            safe_version: "1.3.0".to_owned(),
            abi_json: String::new(),
            abi_method: String::new(),
            abi_args: String::new(),
            manual_signer: String::new(),
            manual_signature: String::new(),
            confirm_signature: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageFormState {
    pub method: String,
    pub payload: String,
    pub threshold: String,
    pub safe_version: String,
    pub manual_signer: String,
    pub manual_signature: String,
}

impl Default for MessageFormState {
    fn default() -> Self {
        Self {
            method: "personal_sign".to_owned(),
            payload: "{}".to_owned(),
            threshold: "1".to_owned(),
            safe_version: "1.3.0".to_owned(),
            manual_signer: String::new(),
            manual_signature: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WalletConnectSurfaceState {
    pub active_topic: String,
    pub request_id: String,
    pub response_json: String,
    pub deferred: bool,
    pub seed_chain_id: String,
}

impl Default for WalletConnectSurfaceState {
    fn default() -> Self {
        Self {
            active_topic: String::new(),
            request_id: String::new(),
            response_json: "{}".to_owned(),
            deferred: false,
            seed_chain_id: "1".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImportExportSurfaceState {
    pub import_bundle_json: String,
    pub url_key: String,
    pub url_payload: String,
    pub export_flow_ids_csv: String,
    pub export_result_json: String,
}

impl Default for ImportExportSurfaceState {
    fn default() -> Self {
        Self {
            import_bundle_json: "{}".to_owned(),
            url_key: "importTx".to_owned(),
            url_payload: String::new(),
            export_flow_ids_csv: String::new(),
            export_result_json: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SigningSurface, SigningUiState};

    #[test]
    fn signing_ui_state_defaults_to_queue_surface() {
        let state = SigningUiState::default();
        assert_eq!(state.active_surface, SigningSurface::Queue);
        assert_eq!(state.active_chain_id, 1);
    }

    #[test]
    fn setting_error_clears_info_and_vice_versa() {
        let mut state = SigningUiState::default();
        state.set_info("ok");
        assert_eq!(state.last_info.as_deref(), Some("ok"));
        state.set_error("bad");
        assert_eq!(state.last_error.as_deref(), Some("bad"));
        assert!(state.last_info.is_none());
    }
}
