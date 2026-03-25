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
        let mut out = [0u8; 8];
        for (i, byte) in self.peer_id.as_bytes().iter().copied().enumerate() {
            // Tiny deterministic hash/fold into 8 bytes for compact BLE payloads.
            let idx = i % 8;
            out[idx] = out[idx].wrapping_mul(31).wrapping_add(byte);
        }
        out
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
