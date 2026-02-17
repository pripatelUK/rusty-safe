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
}

impl Default for SigningUiState {
    fn default() -> Self {
        Self {
            active_surface: SigningSurface::Queue,
            selected_flow_id: None,
        }
    }
}
