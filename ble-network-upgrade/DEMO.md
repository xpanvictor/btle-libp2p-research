# BLE to MultiPeer Connectivity Upgrade Demo

## Overview

This demonstration showcases a complete transport upgrade flow between two Bluetooth Low Energy (BLE) devices, showing why a BLE specification is essential in libp2p's multi-transport architecture.

The demo illustrates:
1. **Discovery** - Nodes discover each other over BLE
2. **Initial Connection** - Establish connection over BLE with identity exchange
3. **Capability Negotiation** - Exchange supported transport capabilities
4. **Transport Upgrade** - Negotiate and switch to a more performant transport (MultiPeer Connectivity)
5. **Resilience** - Maintain BLE as fallback while using upgraded transport

## Architecture

### Modules

#### `identity.rs` - Node Identity
- Generates libp2p-compatible identities for each node
- Uses Ed25519 keypairs for cryptographic identity
- Provides short ID representation (8 bytes) suitable for BLE advertisements (limited to 31 bytes)

```rust
let identity = NodeIdentity::generate();
println!("Peer ID: {}", identity.peer_id);
println!("Short ID for BLE: {:02x?}", identity.short_id());
```

#### `protocol.rs` - Message Protocol
Defines the conversation protocol between peers:

- **Hello** - Initial peer introduction with capabilities
- **HelloAck** - Acknowledgment with peer's capabilities
- **ProposeUpgrade** - Suggest transport upgrade
- **AcceptUpgrade / RejectUpgrade** - Accept or reject upgrade proposal
- **Heartbeat** - Keep-alive messages
- **TransportSwitchNotifier** - Signal successful transport switch
- **Data** - Application payload

```rust
pub enum ProtocolMessage {
    Hello {
        peer_id_short: [u8; 8],
        capabilities: Vec<TransportCapability>,
        max_message_size: u16,
    },
    ProposeUpgrade {
        proposed_transport: TransportCapability,
        upgrade_token: u32,
    },
    // ... other variants
}
```

#### `ble.rs` - BLE Transport Layer
Manages Bluetooth Low Energy operations using btleplug:

- Device discovery and scanning
- Connection management
- Event handling (discovered, connected, disconnected)

```rust
let mut ble_manager = BleManager::new().await?;
ble_manager.start_scan().await?;

while let Some(event) = ble_manager.poll_events().await? {
    match event {
        BleEvent::PeerDiscovered { address, rssi, .. } => {
            println!("Found peer at {}", address);
        }
        // ...
    }
}
```

#### `transport.rs` - Transport Manager
Handles transport state and upgrades:

- Tracks active transport (BLE vs MultiPeer Connectivity)
- Manages transport upgrade operations
- Maintains fallback to BLE
- Multiplexes data over current transport

```rust
let manager = TransportManager::new(); // Starts with BLE
manager.upgrade_to_multipeer("peer_id").await?;
manager.send_data(&payload).await?;
```

## Why BLE Spec is Needed in libp2p

### 1. **Discovery Mechanism**
- BLE advertisements work with very limited payload (31 bytes maximum)
- Requires a compact encoding scheme for peer identity and capability advertisement
- Other transports like TCP rely on DNS or manual addresses - not viable for spontaneous local discovery

### 2. **Initial Bootstrap**
- Not all peers support advanced transports (MultiPeer, Bluetooth BR/EDR, etc.)
- BLE is nearly universal on modern smartphones, tablets, and computers
- Provides guaranteed initial connectivity for identity exchange

### 3. **Transport Negotiation Protocol**
- BLE's small message size necessitates a well-defined negotiation protocol
- Allows peers to discover mutual capabilities before upgrading
- Defines fallback behavior when upgrades fail

### 4. **Connectivity Resilience**
- Keeps BLE as fallback when upgraded transports fail
- Enables seamless recovery during transient network issues
- Provides multi-path communication for reliability

### 5. **Cross-Platform Compatibility**
- BLE works natively on all major platforms (Linux, macOS, Windows, iOS, Android)
- Platform-specific optimizations don't break compatibility
- MultiPeer Connectivity is Apple-only; BLE provides universal baseline

## Flow Diagram

```
┌─────────────────────────────────────────────────────┐
│ Phase 1: Discovery & Connection over BLE            │
├─────────────────────────────────────────────────────┤
│ Node A scans → Discovers Node B (BLE advertisement) │
│ Node A invites → Node B accepts                      │
│ BLE session established                             │
└─────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────┐
│ Phase 2: Identity & Capability Exchange             │
├─────────────────────────────────────────────────────┤
│ Node A → Hello(identity, [BLE, MultiPeer])          │
│ Node B → HelloAck(identity, [BLE, MultiPeer])       │
│ Both support MultiPeer Connectivity (optimal!)      │
└─────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────┐
│ Phase 3: Transport Upgrade Negotiation              │
├─────────────────────────────────────────────────────┤
│ Node A → ProposeUpgrade(MultiPeer, token=123)       │
│ Node B → AcceptUpgrade(token=123)                   │
│ Establishing MultiPeer Connectivity...              │
└─────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────┐
│ Phase 4: High-Bandwidth Communication               │
├─────────────────────────────────────────────────────┤
│ Node A ←→ Node B (larger messages, lower latency)   │
│ MultiPeer Connectivity active                       │
│ BLE maintained as fallback                          │
└─────────────────────────────────────────────────────┘
```

## Usage

### Building

```bash
cd ble-network-upgrade
cargo build --release
```

### Running the Demo

```bash
# Run on first Mac
./target/release/ble-network-upgrade

# Run on second Mac (in parallel, same network)
./target/release/ble-network-upgrade
```

Note: If BLE hardware is not available, the demo runs in simulated mode showing the expected flow.

### Output Example

```
╔════════════════════════════════════════════════════════════╗
║  BLE -> MultiPeer Connectivity Transport Upgrade Demo      ║
║  Demonstrating why BLE spec is needed in libp2p            ║
╚════════════════════════════════════════════════════════════╝

[Node] Identity generated: 12ab34cd56ef78...
[Node] Short ID (for BLE advertisement): [0x12, 0xab, 0x34, 0xcd, ...]

[BLE] Starting scan for nearby devices...

[Discovery] Found peer: AA:BB:CC:DD:EE:FF (RSSI: -50 dBm)

[Discovery Phase Complete] Found 1 peer(s)

┌────────────────────────────────────────────────────────────┐
│  Phase 1: Initial Connection over BLE                      │
└────────────────────────────────────────────────────────────┘

[BLE] Sending Hello message with capabilities...
  - Transport: BLE
  - Capabilities: Hello { peer_id_short: [...], capabilities: [Ble, MultiPeer], max_message_size: 512 }
  - Message size: 156 bytes

[BLE Session] Exchanging small messages...
[Transport] Sending 17 bytes over BLE
[Transport] Sending 30 bytes over BLE
[Transport] Sending 23 bytes over BLE

┌────────────────────────────────────────────────────────────┐
│  Phase 2: Capability Negotiation                           │
└────────────────────────────────────────────────────────────┘

[Negotiation] Both peers support MultiPeer Connectivity
[Negotiation] Advantages of MultiPeer over BLE:
  ✓ Higher bandwidth (faster data transfer)
  ✓ More stable connection
  ✓ Better for larger payloads
  ✓ Lower latency
  ✗ Only available on Apple platforms
  ✗ Requires BLE as fallback mechanism

...

╔════════════════════════════════════════════════════════════╗
║  Demo Complete: Transport Upgrade Flow Validated           ║
╚════════════════════════════════════════════════════════════╝
```

## Key Implementation Points

### Message Protocol
Messages are JSON-encoded for clarity and debuggability:

```json
{
  "Hello": {
    "peer_id_short": [1, 2, 3, 4, 5, 6, 7, 8],
    "capabilities": ["Ble", "MultiPeerConnectivity"],
    "max_message_size": 512
  }
}
```

### Transport Capabilities
Extensible enum for future transport types:

```rust
pub enum TransportCapability {
    Ble,                        // Always available, baseline
    MultiPeerConnectivity,      // macOS/iOS specific, higher bandwidth
    SmartBluetooth,             // Future: Bluetooth BR/EDR
}
```

### Async Architecture
Uses Tokio for async operations:
- Non-blocking BLE scanning and connect/disconnect
- Concurrent event handling
- Clean async/await patterns for transport upgrade

## Testing

Run the test suite:

```bash
cargo test
```

The test suite validates:
- Identity generation and ID derivation
- Message serialization/deserialization
- Protocol message handling
- Transport upgrade logic
- Fallback mechanisms

## Design Rationale

### Why Multiple Transports?
1. **No single transport is optimal** - BLE excels at discovery, MultiPeer at throughput
2. **Platform availability varies** - BLE is universal, newer transports are platform-specific
3. **Graceful degradation** - Always have a fallback, upgrade when possible

### Why BLE Must Be a Spec?
1. **Limited advertisement payload** - Can't fit everything in 31 bytes; needs compact encoding
2. **Energy constraints** - Scanning is expensive; needs efficient discovery mechanism
3. **Cross-platform coordination** - All implementations need to speak the same language
4. **Network discovery** - Only BLE provides automatic local peer discovery

### Why Not Just Use One Transport?
- **Performance**: BLE alone can't handle large messages efficiently
- **Universality**: Not all platforms support all transports
- **Resilience**: Single failure point becomes critical
- **Flexibility**: Allows application-level optimization choices

## Future Enhancements

1. **Actual MultiPeer Implementation** - Use `objc2-multipeer-connectivity` for real connections
2. **Noise Protocol** - Add cryptographic security to all transports
3. **NAT Traversal** - Support transports like relay and mapping
4. **Service Discovery** - Advertise services, not just peers
5. **Bandwidth Adaptation** - Dynamically select transport based on measured bandwidth
6. **Multi-Hop Routing** - Route messages through multiple peers

## References

- [libp2p Transports](https://docs.libp2p.io/concepts/transports/)
- [BLE Specification](https://www.bluetooth.com/specifications/specs/)
- [btleplug Documentation](https://crates.io/crates/btleplug)
- [libp2p Identity and Keys](https://docs.libp2p.io/concepts/peer-id/)

## License

Part of the btle-libp2p-blog research project.
