/// Identity module - generates and manages libp2p identities
use libp2p::{identity::Keypair, PeerId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeIdentity {
    pub peer_id: String,
    pub public_key_bytes: Vec<u8>,
}

impl NodeIdentity {
    /// Generate a new random identity
    pub fn generate() -> Self {
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        let peer_id_str = peer_id.to_string();
        let public_key_bytes = keypair.public().encode_protobuf().to_vec();

        Self {
            peer_id: peer_id_str,
            public_key_bytes,
        }
    }

    /// Get a shorter representation for BLE advertisement (limited to 31 bytes)
    pub fn short_id(&self) -> [u8; 8] {
        let mut short = [0u8; 8];
        let peer_bytes = self.peer_id.as_bytes();
        let copy_len = std::cmp::min(8, peer_bytes.len());
        short[..copy_len].copy_from_slice(&peer_bytes[..copy_len]);
        short
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let identity = NodeIdentity::generate();
        assert!(!identity.public_key_bytes.is_empty());
        assert_eq!(identity.short_id().len(), 8);
    }
}
