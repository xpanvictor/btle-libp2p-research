/// Protocol module - defines message types for BLE communication and capability negotiation
use serde::{Deserialize, Serialize};
use std::fmt;

/// Capabilities that a node supports for transport upgrade
#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportCapability {
    /// BLE only
    Ble,
    /// MultiPeer Connectivity (macOS/iOS specific)
    MultiPeerConnectivity,
    /// Future: Bluetooth BR/EDR
    SmartBluetooth,
}

impl fmt::Display for TransportCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportCapability::Ble => write!(f, "BLE"),
            TransportCapability::MultiPeerConnectivity => write!(f, "MultiPeer"),
            TransportCapability::SmartBluetooth => write!(f, "SmartBluetooth"),
        }
    }
}

/// Protocol message types for BLE communication
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProtocolMessage {
    /// Initial hello with identity and capabilities
    Hello {
        peer_id_short: [u8; 8],
        capabilities: Vec<TransportCapability>,
        max_message_size: u16,
    },
    /// Acknowledge connection
    HelloAck {
        peer_id_short: [u8; 8],
    },
    /// Propose transport upgrade
    ProposeUpgrade {
        proposed_transport: TransportCapability,
        upgrade_token: u32,
    },
    /// Accept upgrade proposal
    AcceptUpgrade {
        upgrade_token: u32,
    },
    /// Reject upgrade proposal
    RejectUpgrade {
        upgrade_token: u32,
        reason: String,
    },
    /// Heartbeat to keep connection alive
    Heartbeat,
    /// User application data
    Data(Vec<u8>),
    /// Indicates transition to new transport
    TransportSwitchNotifier {
        new_transport: TransportCapability,
    },
}

impl ProtocolMessage {
    /// Encode message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(serde_json::to_vec(self)?)
    }

    /// Decode message from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(data)?)
    }

    /// Get a human-readable label for logging
    pub fn label(&self) -> &'static str {
        match self {
            ProtocolMessage::Hello { .. } => "Hello",
            ProtocolMessage::HelloAck { .. } => "HelloAck",
            ProtocolMessage::ProposeUpgrade { .. } => "ProposeUpgrade",
            ProtocolMessage::AcceptUpgrade { .. } => "AcceptUpgrade",
            ProtocolMessage::RejectUpgrade { .. } => "RejectUpgrade",
            ProtocolMessage::Heartbeat => "Heartbeat",
            ProtocolMessage::Data(_) => "Data",
            ProtocolMessage::TransportSwitchNotifier { .. } => "TransportSwitch",
        }
    }
}

/// Represents a discovered peer
#[derive(Clone, Debug)]
pub struct DiscoveredPeer {
    pub peer_id_short: [u8; 8],
    pub capabilities: Vec<TransportCapability>,
    pub max_message_size: u16,
    pub is_connected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = ProtocolMessage::Hello {
            peer_id_short: [1, 2, 3, 4, 5, 6, 7, 8],
            capabilities: vec![TransportCapability::Ble, TransportCapability::MultiPeerConnectivity],
            max_message_size: 512,
        };

        let bytes = msg.to_bytes().unwrap();
        let decoded = ProtocolMessage::from_bytes(&bytes).unwrap();

        match decoded {
            ProtocolMessage::Hello {
                capabilities,
                max_message_size,
                ..
            } => {
                assert_eq!(capabilities.len(), 2);
                assert_eq!(max_message_size, 512);
            }
            _ => panic!("Wrong message type"),
        }
    }
}
