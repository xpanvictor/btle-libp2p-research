# Quick Reference Guide

## Running the Demo

### Simulated Mode (No Hardware Required)
```bash
cd ble-network-upgrade
cargo run
```

### On Actual Hardware (macOS with Bluetooth)
```bash
# Terminal 1 (First Mac)
cargo run --release

# Terminal 2 (Second Mac)
cargo run --release
```

## Code Snippets

### 1. Generate Node Identity
```rust
use ble_network_upgrade::identity::NodeIdentity;

let identity = NodeIdentity::generate();
println!("Peer ID: {}", identity.peer_id);
println!("Short ID: {:02x?}", identity.short_id());
```

### 2. Initialize BLE Manager
```rust
use ble_network_upgrade::ble::BleManager;

let mut ble = BleManager::new().await?;
ble.start_scan().await?;
```

### 3. Poll for BLE Events
```rust
use ble_network_upgrade::ble::BleEvent;

while let Some(event) = ble.poll_events().await? {
    match event {
        BleEvent::PeerDiscovered { address, rssi, .. } => {
            println!("Found: {} at {}", address, rssi);
        }
        BleEvent::PeerConnected { address } => {
            println!("Connected: {}", address);
        }
        _ => {}
    }
}
```

### 4. Create Protocol Messages
```rust
use ble_network_upgrade::protocol::{ProtocolMessage, TransportCapability};

// Hello message
let hello = ProtocolMessage::Hello {
    peer_id_short: [0; 8],
    capabilities: vec![
        TransportCapability::Ble,
        TransportCapability::MultiPeerConnectivity,
    ],
    max_message_size: 512,
};

// Serialize
let bytes = hello.to_bytes()?;

// Deserialize
let msg = ProtocolMessage::from_bytes(&bytes)?;
```

### 5. Manage Transport Upgrades
```rust
use ble_network_upgrade::transport::TransportManager;

let mgr = TransportManager::new();

// On BLE initially
mgr.send_data(b"Small message").await?;

// Upgrade
mgr.upgrade_to_multipeer("peer123").await?;

// Now using MultiPeer
mgr.send_data(b"Large message over fast transport").await?;
```

## Performance Targets

| Operation | Expected Time | Notes |
|-----------|---------------|-------|
| Scan for peers | 1-5 seconds | Depends on area volume |
| Discover single peer | <1 second | If already advertising |
| Connect to peer | 2-5 seconds | BLE connection time |
| Exchange hello | <100ms | Small JSON payload |
| Negotiate upgrade | 100-500ms | Token exchange |
| Upgrade to MultiPeer | <1 second | System-dependent |
| Send 1KB over BLE | 100-500ms | Multiple packets |
| Send 1KB over MultiPeer | <10ms | Single packet |

## Testing Checklist

- [ ] `cargo check` passes without errors
- [ ] `cargo test` passes all unit tests
- [ ] `cargo build --release` succeeds
- [ ] `cargo run` works in simulated mode
- [ ] Output shows all phases (Discovery → Negotiation → Upgrade → Communication)
- [ ] Protocol messages serialize/deserialize correctly
- [ ] Transport manager switches transports successfully

## Common Issues

### Issue: "No BLE adapter found"
**Solution**: Project switches to simulated mode. This is normal on systems without Bluetooth hardware.

### Issue: "btleplug compile errors"
**Solution**: Ensure you have the latest Rust:
```bash
rustup update
cargo clean
cargo build
```

### Issue: Tests fail with timing issues
**Solution**: Increase timeout in `Cargo.toml`:
```toml
[profile.test]
opt-level = 0
```

### Issue: Large JSON messages
**Solution**: For production, use Protobuf instead:
```toml
prost = "0.12"
```

## Project Structure
```
ble-network-upgrade/
├── Cargo.toml                 # Manifest with dependencies
├── src/
│   ├── main.rs               # Main demo orchestration
│   ├── identity.rs           # Peer identity generation
│   ├── protocol.rs           # Message protocol definitions
│   ├── ble.rs               # BLE transport layer
│   └── transport.rs         # Transport upgrade manager
├── target/                   # Build artifacts
├── DEMO.md                   # Comprehensive demo guide
├── IMPLEMENTATION_GUIDE.md   # Code examples and patterns
├── WHY_BLE_MATTERS.md       # Deep dive on BLE in libp2p
└── README.md                # Quick start (this file)
```

## Key Concepts

### Protocol Messages
- **Hello**: "Here's my identity and what I support"
- **HelloAck**: "Got it, here's mine"
- **ProposeUpgrade**: "Let's use this faster transport"
- **AcceptUpgrade**: "Sounds good, switching now"
- **Heartbeat**: "Still here?"
- **TransportSwitchNotifier**: "I'm now on the new transport"

### Transport Capabilities
```
BLE                          - Always available
MultiPeerConnectivity        - macOS/iOS high-bandwidth
SmartBluetooth (future)      - Bluetooth BR/EDR
```

### State Management
1. **Discovery** - Scanning for peers
2. **Connection** - Connected via BLE
3. **Exchange** - Sharing Hello messages  
4. **Negotiation** - Capability matching
5. **Upgrade** - Switching transport
6. **Active** - Using new transport with BLE fallback

## Next Steps

1. **Deep Dive**: Read `WHY_BLE_MATTERS.md` for technical justification
2. **Implementation Details**: Check `IMPLEMENTATION_GUIDE.md` for code patterns
3. **Full Demo**: Run `cargo run` and watch all phases in action
4. **Extend**: Add new transport types or message types
5. **Integrate**: Use in your own libp2p application

## Resources

- [libp2p Docs](https://docs.libp2p.io/)
- [btleplug Docs](https://docs.rs/btleplug/)
- [Bluetooth Spec](https://www.bluetooth.com/)
- [Tokio Async Runtime](https://tokio.rs/)

## Support

For issues or questions:
1. Check the implementation guide for examples
2. Review `WHY_BLE_MATTERS.md` for design rationale
3. Examine `src/main.rs` for the full demo flow
4. Run tests: `cargo test -- --nocapture` for detailed output
