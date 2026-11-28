//! `tiles server start` and `tiles server stop`

use anyhow::Result;

use crate::api::DaemonApi;

/// `tiles server start`: starts the inference server through the daemon
pub async fn start_server(api: &DaemonApi) -> Result<()> {
    println!("{}", api.start_server().await?);
    Ok(())
}

/// `tiles server stop`: stops the inference server through the daemon
pub async fn stop_server(api: &DaemonApi) -> Result<()> {
    println!("{}", api.stop_server().await?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::api::ServerState;

    #[test]
    fn server_states_format_as_expected() {
        assert_eq!(ServerState::Started.to_string(), "Inference server started");
        assert_eq!(
            ServerState::AlreadyRunning.to_string(),
            "Inference server is already running"
        );
        assert_eq!(ServerState::Stopped.to_string(), "Inference server stopped");
        assert_eq!(
            ServerState::NotRunning.to_string(),
            "Inference server is not running"
        );
    }
}
