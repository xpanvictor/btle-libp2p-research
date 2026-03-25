/// Multipeer Connectivity module - handles transport upgrade to multipeer connectivity
use std::sync::Arc;
use tokio::sync::Mutex;

/// Represents the multiplexed connection with either BLE or Multipeer transport
#[derive(Clone)]
pub enum ActiveTransport {
    Ble,
    MultiPeerConnectivity,
}

/// Multipeer connection info
#[derive(Clone, Debug)]
pub struct MultiPeerSession {
    pub peer_id: String,
    pub is_secure: bool,
}

/// Manages transport upgrade and dual-transport scenarios
pub struct TransportManager {
    active_transport: Arc<Mutex<ActiveTransport>>,
    current_session: Arc<Mutex<Option<MultiPeerSession>>>,
    ble_fallback_enabled: bool,
}

impl TransportManager {
    /// Create new transport manager
    pub fn new() -> Self {
        Self {
            active_transport: Arc::new(Mutex::new(ActiveTransport::Ble)),
            current_session: Arc::new(Mutex::new(None)),
            ble_fallback_enabled: true,
        }
    }

    /// Get current active transport
    pub async fn get_active_transport(&self) -> ActiveTransport {
        self.active_transport.lock().await.clone()
    }

    /// Upgrade to multipeer connectivity
    pub async fn upgrade_to_multipeer(
        &self,
        peer_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "[Transport] Upgrading connection to {} to MultiPeer Connectivity",
            peer_id
        );

        // Simulate multipeer connectivity handshake
        let session = MultiPeerSession {
            peer_id: peer_id.to_string(),
            is_secure: true,
        };

        *self.current_session.lock().await = Some(session);
        *self.active_transport.lock().await = ActiveTransport::MultiPeerConnectivity;

        println!("[Transport] Successfully upgraded to MultiPeer Connectivity");
        Ok(())
    }

    /// Fallback to BLE
    pub async fn fallback_to_ble(&self) {
        println!("[Transport] Falling back to BLE (MultiPeer unavailable)");
        *self.active_transport.lock().await = ActiveTransport::Ble;
    }

    /// Send data on current transport
    pub async fn send_data(&self, data: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
        let transport = self.get_active_transport().await;

        match transport {
            ActiveTransport::Ble => {
                println!("[Transport] Sending {} bytes over BLE", data.len());
                Ok(data.len())
            }
            ActiveTransport::MultiPeerConnectivity => {
                println!(
                    "[Transport] Sending {} bytes over MultiPeer Connectivity",
                    data.len()
                );
                Ok(data.len())
            }
        }
    }

    /// Get current session info
    pub async fn get_session(&self) -> Option<MultiPeerSession> {
        self.current_session.lock().await.clone()
    }

    /// Check if BLE fallback is available
    pub fn is_ble_fallback_enabled(&self) -> bool {
        self.ble_fallback_enabled
    }
}

impl Default for TransportManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transport_upgrade() {
        let manager = TransportManager::new();

        // Initially BLE
        assert!(matches!(
            manager.get_active_transport().await,
            ActiveTransport::Ble
        ));

        // Upgrade to MultiPeer
        manager
            .upgrade_to_multipeer("test_peer")
            .await
            .expect("Upgrade failed");

        assert!(matches!(
            manager.get_active_transport().await,
            ActiveTransport::MultiPeerConnectivity
        ));

        // Check session
        let session = manager.get_session().await;
        assert!(session.is_some());
    }

    #[tokio::test]
    async fn test_fallback_to_ble() {
        let manager = TransportManager::new();

        manager
            .upgrade_to_multipeer("test_peer")
            .await
            .expect("Upgrade failed");
        manager.fallback_to_ble().await;

        assert!(matches!(
            manager.get_active_transport().await,
            ActiveTransport::Ble
        ));
    }
}
