# Implementation Guide

## Getting Started

### Prerequisites

- Rust 1.70+
- macOS 10.13+ (for actual BLE functionality) or any OS (for simulated demo)
- Cargo

### Quick Start

```bash
# Clone and navigate to the project
cd ble-network-upgrade

# Run in debug mode (simulated demo)
cargo run

# Build optimized version
cargo build --release

# Run tests
cargo test
```

## Module Documentation

### Identity Module

The identity module handles peer identification using libp2p's PeerId:

```rust
use ble_network_upgrade::identity::NodeIdentity;

// Generate a new identity
let identity = NodeIdentity::generate();

// Access components
println!("Peer ID: {}", identity.peer_id);
println!("Public Key: {:?}", identity.public_key_bytes);
println!("Short ID (8 bytes): {:02x?}", identity.short_id());
```

**Key Design Decision**: We store peer_id as a String for Serde compatibility with libp2p's types.

### Protocol Module

The protocol module defines message types for peer communication:

```rust
use ble_network_upgrade::protocol::{ProtocolMessage, TransportCapability};

// Create a Hello message
let hello = ProtocolMessage::Hello {
    peer_id_short: [1, 2, 3, 4, 5, 6, 7, 8],
    capabilities: vec![
        TransportCapability::Ble,
        TransportCapability::MultiPeerConnectivity,
    ],
    max_message_size: 512,
};

// Serialize to JSON
let json_bytes = hello.to_bytes()?;

// Deserialize from bytes
let decoded = ProtocolMessage::from_bytes(&json_bytes)?;
```

**Message Types**:
- `Hello` - Initial introduction with capabilities
- `HelloAck` - Acknowledgment
- `ProposeUpgrade` - Suggest transport change
- `AcceptUpgrade` / `RejectUpgrade` - Respond to upgrade
- `Heartbeat` - Keep-alive signal
- `Data` - Application payload
- `TransportSwitchNotifier` - Announce successful switch

### BLE Module

The BLE module handles Bluetooth Low Energy operations:

```rust
use ble_network_upgrade::ble::{BleManager, BleEvent};

// Initialize the BLE manager
let mut ble = BleManager::new().await?;

// Start scanning
ble.start_scan().await?;

// Poll for events
loop {
    if let Some(event) = ble.poll_events().await? {
        match event {
            BleEvent::PeerDiscovered { address, rssi, adv_data } => {
                println!("Found: {} (signal: {} dBm)", address, rssi);
            }
            BleEvent::PeerConnected { address } => {
                println!("Connected to: {}", address);
            }
            BleEvent::PeerDisconnected { address } => {
                println!("Disconnected from: {}", address);
            }
        }
    }
}

// Stop scanning when done
ble.stop_scan().await?;
```

**Note**: Event handling in the current implementation is channel-based for simulation. Real implementation would integrate with btleplug's event stream.

### Transport Module

The transport module manages transport switching:

```rust
use ble_network_upgrade::transport::TransportManager;

// Create manager (initializes with BLE)
let transport = TransportManager::new();

// Check current transport
let active = transport.get_active_transport().await;

// Send data over current transport
transport.send_data(b"Hello").await?;

// Upgrade to MultiPeer
transport.upgrade_to_multipeer("peer_id").await?;

// Now all send_data calls use MultiPeer
transport.send_data(b"Larger message over MultiPeer").await?;

// Can retrieve session info
if let Some(session) = transport.get_session().await {
    println!("Connected via MultiPeer to: {}", session.peer_id);
}

// Fallback to BLE if needed
transport.fallback_to_ble().await;
```

## Architecture Patterns

### Async/Await for Non-Blocking Operations

All I/O operations are async:

```rust
let manager = BleManager::new().await?;  // Async initialization
manager.start_scan().await?;              // Async scanning
while let Some(event) = manager.poll_events().await? {
    // Handle events
}
```

### Channel-Based Event Distribution

Events are distributed through Tokio channels:

```rust
let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

// Sender can send from anywhere
tx.send(BleEvent::PeerDiscovered { ... })?;

// Receiver polls for events
if let Ok(event) = rx.try_recv() {
    // Handle event
}
```

### State Machine for Transport Selection

Transport state is wrapped in Arc<Mutex<>>:

```rust
let active_transport = Arc::new(Mutex::new(ActiveTransport::Ble));

// Different tasks can modify state
{
    let mut state = active_transport.lock().await;
    *state = ActiveTransport::MultiPeerConnectivity;
}
```

## Error Handling

The modules use `Result<T, Box<dyn std::error::Error>>` for flexible error handling:

```rust
// Chain errors naturally
let ble = BleManager::new().await?;
ble.start_scan().await?;
ble.connect("AA:BB:CC:DD:EE:FF").await?;

// Or handle specific cases
match ble.poll_events().await {
    Ok(Some(event)) => { /* handle */ }
    Ok(None) => { /* no event */ }
    Err(e) => println!("Error: {}", e),
}
```

## Performance Characteristics

### BLE Transport
- **Message Size**: ~20 bytes typical payload
- **Latency**: 7.5ms-10ms per packet
- **Bandwidth**: ~1 Mbps practical
- **Power**: Very low

### MultiPeer Connectivity (Target)
- **Message Size**: ~10KB typical payload
- **Latency**: <1ms per packet
- **Bandwidth**: ~10 Mbps (LAN speeds)
- **Power**: Moderate

## Testing Strategy

### Unit Tests

Test individual components:

```bash
cargo test identity::tests
cargo test protocol::tests
cargo test transport::tests
```

### Integration Tests

Test multi-module interactions:

```bash
cargo test  # Runs all tests including integration
```

### Manual Testing

Run the full demo:

```bash
# Terminal 1
cargo run

# Terminal 2 (on another Mac if possible)
cargo run
```

## Debugging

### Enable Logging

Add to your code:

```rust
env_logger::init();
```

Run with:

```bash
RUST_LOG=debug cargo run
```

### Check Protocol Messages

The protocol module includes debug output:

```rust
let msg = ProtocolMessage::Hello { /* ... */ };
println!("Message: {:?}", msg);           // Full debug
println!("Type: {}", msg.label());        // Just the type name
println!("Bytes: {:?}", msg.to_bytes()?); // JSON representation
```

## Extending the Demo

### Adding a New Transport

1. Create a new enum variant in `protocol.rs`:
```rust
pub enum TransportCapability {
    Ble,
    MultiPeerConnectivity,
    MyNewTransport,  // Add here
}
```

2. Add message type in protocol:
```rust
pub enum ProtocolMessage {
    // ... existing types
    CustomMessage { /* your fields */ },
}
```

3. Implement handler in main.rs:
```rust
match message {
    ProtocolMessage::CustomMessage { /* ... */ } => {
        // Handle your message
    }
    _ => {}
}
```

### Adding Encryption

Integrate libp2p's Noise protocol:

```rust
use libp2p::noise;

// In identity generation
let noise_config = noise::Config::default()
    .with_local_private_key(keypair.to_protobuf_encoding()?);
```

### Adding Service Discovery

Extend protocol with service advertisements:

```rust
pub enum ProtocolMessage {
    // ... existing
    AdvertiseService {
        service_id: String,
        capabilities: Vec<String>,
    },
}
```

## Known Limitations

1. **No Encryption**: Current implementation demonstrates protocol, not security
2. **Single Adapter**: Only uses first available Bluetooth adapter
3. **No Persistence**: Peer information not stored between runs
4. **Mock MultiPeer**: MultiPeer upgrade not actually implemented (placeholder)
5. **Limited Payload**: BLE tests use simulated messages, not actual BLE packets

## Future Work

1. **Real MultiPeer Implementation** - Actually use `objc2-multipeer-connectivity`
2. **Noise Protocol** - Add cryptographic authentication
3. **Persistent Storage** - Remember peers across sessions
4. **Multiple Adapters** - Support multiple BLE devices
5. **Platform Abstraction** - Support all platforms (Linux, Windows, etc.)
6. **Formal Specification** - Document as libp2p transport spec

## References

- [Tokio Documentation](https://tokio.rs/)
- [btleplug Examples](https://github.com/deviceplug/btleplug/tree/master/examples)
- [libp2p Development Guide](https://docs.libp2p.io/)
- [Bluetooth Spec Overview](https://www.bluetooth.com/technical-specifications-documentation/)
