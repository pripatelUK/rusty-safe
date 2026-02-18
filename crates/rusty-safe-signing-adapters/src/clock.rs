use rusty_safe_signing_core::{ClockPort, PortError};

#[derive(Debug, Clone, Default)]
pub struct SystemClockAdapter;

impl ClockPort for SystemClockAdapter {
    fn now_ms(&self) -> Result<u64, PortError> {
        #[cfg(target_arch = "wasm32")]
        {
            let now = web_time::SystemTime::now()
                .duration_since(web_time::UNIX_EPOCH)
                .map_err(|e| PortError::Transport(format!("time error: {e}")))?;
            return Ok(now.as_millis() as u64);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| PortError::Transport(format!("time error: {e}")))?;
            Ok(now.as_millis() as u64)
        }
    }
}
