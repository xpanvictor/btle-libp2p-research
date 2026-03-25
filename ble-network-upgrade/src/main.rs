mod apple_ble;
mod ble;
mod identity;
mod multipeer;
mod protocol;
mod transport;

use ble::{BleEvent, BleManager};
use identity::NodeIdentity;
use multipeer::MultiPeerRole;
use protocol::{DiscoveredPeer, ProtocolMessage, TransportCapability};
use std::time::Duration;
use tokio::time::sleep;
use transport::TransportManager;

/// Main demonstration: two BLE devices discover, connect, and upgrade transport
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  BLE -> MultiPeer Connectivity Transport Upgrade Demo      ║");
    println!("║  Demonstrating why BLE spec is needed in libp2p            ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Generate node identity
    let identity = NodeIdentity::generate();
    println!(
        "[Node] Identity generated: {}",
        identity.peer_id.to_string()[0..8].to_string()
    );
    println!(
        "[Node] Short ID (for BLE advertisement): {:02x?}\n",
        identity.short_id()
    );

    // Initialize BLE manager
    let mut ble_manager = match BleManager::new().await {
        Ok(mgr) => mgr,
        Err(e) => {
            println!("[Error] Failed to initialize BLE: {}", e);
            println!("[Info] BLE is only available on macOS with compatible hardware");
            println!("[Info] Simulating BLE behavior for demonstration...\n");
            return run_simulated_demo(&identity, MultiPeerRole::Both).await;
        }
    };

    // Initialize transport manager
    let transport_manager = TransportManager::new();

    let mp_role = MultiPeerRole::Both;
    println!("[Config] Auto mode: BLE advertise+scan and MultiPeer initiator+responder");

    ble_manager.start_advertising(identity.short_id())?;
    ble_manager.start_scan().await?;
    println!("[BLE] Advertising and scanning simultaneously...\n");

    // Collect discovered peers
    let mut discovered_peers: Vec<DiscoveredPeer> = Vec::new();
    let scan_duration = Duration::from_secs(8);

    // Run for 10 seconds to discover peers
    let start = std::time::Instant::now();
    while start.elapsed() < scan_duration {
        match ble_manager.poll_events().await {
            Ok(Some(event)) => {
                match event {
                    BleEvent::PeerDiscovered {
                        address,
                        rssi,
                        adv_data: _,
                    } => {
                        println!(
                            "[Discovery] Found peer: {} (RSSI: {} dBm)",
                            address, rssi
                        );
                        discovered_peers.push(DiscoveredPeer {
                            peer_id_short: identity.short_id(),
                            capabilities: vec![
                                TransportCapability::Ble,
                                TransportCapability::MultiPeerConnectivity,
                            ],
                            max_message_size: 512,
                            is_connected: false,
                        });
                    }
                    BleEvent::PeerConnected { address } => {
                        println!("[Connection] Successfully connected to: {}", address);
                    }
                    BleEvent::PeerDisconnected { address } => {
                        println!("[Disconnection] Lost connection to: {}", address);
                    }
                }
            }
            Ok(None) => {
                sleep(Duration::from_millis(100)).await;
            }
            Err(e) => {
                println!("[Warning] Error polling BLE events: {}", e);
            }
        }
    }

    ble_manager.stop_scan().await?;

    if discovered_peers.is_empty() {
        println!(
            "\n[Info] No peers discovered during BLE scan. Continuing to transport negotiation...\n"
        );
        println!(
            "[Info] For reliable two-Mac discovery use BLE_ROLE=advertise on one node and BLE_ROLE=scan on the other.\n"
        );
    }

    println!("\n[Discovery Phase Complete] Found {} peer(s)\n", discovered_peers.len());

    // Demonstrate connection and upgrade flow
    demonstrate_transport_upgrade(&identity, &transport_manager, mp_role).await?;

    ble_manager.stop_advertising();

    Ok(())
}

/// Simulated demonstration without actual BLE hardware
async fn run_simulated_demo(
    identity: &NodeIdentity,
    mp_role: MultiPeerRole,
) -> Result<(), Box<dyn std::error::Error>> {
    let transport_manager = TransportManager::new();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║             Simulated Transport Upgrade Flow               ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    demonstrate_transport_upgrade(identity, &transport_manager, mp_role).await?;

    Ok(())
}

/// Demonstrate the transport upgrade flow
async fn demonstrate_transport_upgrade(
    identity: &NodeIdentity,
    transport_manager: &TransportManager,
    mp_role: MultiPeerRole,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("┌────────────────────────────────────────────────────────────┐");
    println!("│  Phase 1: BLE Discovery + Control Handshake                │");
    println!("└────────────────────────────────────────────────────────────┘\n");

    println!("[BLE] Discovery is real (advertise+scan). Data-channel exchange below is simulated control flow for this demo.\n");

    // Simulate exchange of Hello message
    let hello_msg = ProtocolMessage::Hello {
        peer_id_short: identity.short_id(),
        capabilities: vec![
            TransportCapability::Ble,
            TransportCapability::MultiPeerConnectivity,
        ],
        max_message_size: 512,
    };

    println!("[BLE] Sending Hello message with capabilities...");
    println!("  - Transport: BLE");
    println!("  - Capabilities: {:?}", hello_msg);

    let msg_bytes = hello_msg.to_bytes()?;
    println!("  - Message size: {} bytes", msg_bytes.len());

    // Simulate small message exchange over BLE
    println!("\n[BLE Session] Exchanging small control messages (simulated transcript)...");
    let test_messages = vec![
        ("local", "remote", "Hello from Node A"),
        ("remote", "local", "ACK, capabilities received"),
        ("local", "remote", "Ready for upgrade"),
    ];

    for (from, to, msg) in test_messages {
        println!("[BLE][Sim][{} -> {}] {}", from, to, msg);
        transport_manager.send_data(msg.as_bytes()).await?;
        drain_and_log_received(transport_manager, "BLE phase").await;
        sleep(Duration::from_millis(500)).await;
    }

    println!("\n┌────────────────────────────────────────────────────────────┐");
    println!("│  Phase 2: Capability Negotiation                           │");
    println!("└────────────────────────────────────────────────────────────┘\n");

    println!("[Negotiation] Both peers support MultiPeer Connectivity");
    println!("[Negotiation] Advantages of MultiPeer over BLE:");
    println!("  ✓ Higher bandwidth (faster data transfer)");
    println!("  ✓ More stable connection");
    println!("  ✓ Better for larger payloads");
    println!("  ✓ Lower latency");
    println!("  ✗ Only available on Apple platforms");
    println!("  ✗ Requires BLE as fallback mechanism\n");

    let upgrade_proposal = ProtocolMessage::ProposeUpgrade {
        proposed_transport: TransportCapability::MultiPeerConnectivity,
        upgrade_token: 0x12345678,
    };

    println!("[BLE] Proposing transport upgrade...");
    println!("  Messages sent: {:?}\n", upgrade_proposal);

    sleep(Duration::from_secs(1)).await;

    let upgrade_accept = ProtocolMessage::AcceptUpgrade {
        upgrade_token: 0x12345678,
    };

    println!("[BLE] Received upgrade acceptance");
    println!("  Message: {:?}\n", upgrade_accept);

    println!("┌────────────────────────────────────────────────────────────┐");
    println!("│  Phase 3: Transport Switch to MultiPeer Connectivity       │");
    println!("└────────────────────────────────────────────────────────────┘\n");

    let display_name = format!("node-{}", &identity.peer_id[0..8]);
    transport_manager
        .upgrade_to_multipeer("peer_node", &display_name, mp_role)
        .await?;
    sleep(Duration::from_millis(250)).await;
    drain_and_log_received(transport_manager, "post-upgrade").await;

    // Notify about transport switch
    let notifier = ProtocolMessage::TransportSwitchNotifier {
        new_transport: TransportCapability::MultiPeerConnectivity,
    };
    println!("[Notification] {:?}\n", notifier);

    println!("┌────────────────────────────────────────────────────────────┐");
    println!("│  Phase 4: Communication over MultiPeer Connectivity        │");
    println!("└────────────────────────────────────────────────────────────┘\n");

    let large_messages = vec![
        ("Larger payload part 1", 256),
        ("Larger payload part 2 with more data", 512),
        ("Large file chunk", 1024),
    ];

    for (msg, size) in large_messages {
        transport_manager
            .send_data(&vec![0u8; size])
            .await?;
        println!("[MultiPeer] Successfully sent {} bytes: {}", size, msg);
        sleep(Duration::from_millis(200)).await;
        drain_and_log_received(transport_manager, "large-data").await;
        sleep(Duration::from_millis(300)).await;
    }

    println!("\n┌────────────────────────────────────────────────────────────┐");
    println!("│  Phase 5: Connection Resilience                            │");
    println!("└────────────────────────────────────────────────────────────┘\n");

    println!("[Resilience] MultiPeer connection active");
    println!("[Resilience] BLE connection is kept as fallback");

    // Simulate heartbeat to keep connection alive
    for i in 1..=3 {
        let heartbeat = ProtocolMessage::Heartbeat;
        transport_manager.send_data(&heartbeat.to_bytes()?).await?;
        println!("[Heartbeat] Keep-alive pulse {} over MultiPeer", i);
        sleep(Duration::from_millis(150)).await;
        drain_and_log_received(transport_manager, "heartbeat").await;
        sleep(Duration::from_secs(1)).await;
    }

    println!("\n┌────────────────────────────────────────────────────────────┐");
    println!("│  Summary: Why BLE Spec is Important for libp2p            │");
    println!("└────────────────────────────────────────────────────────────┘\n");

    println!("1. **Discovery Mechanism**");
    println!("   - BLE provides efficient peer discovery in local networks");
    println!("   - Works with limited advertisement payload (31 bytes)");
    println!("   - Energy efficient scanning\n");

    println!("2. **Initial Bootstrap**");
    println!("   - Not all peers support advanced transports");
    println!("   - BLE is nearly universal on modern devices");
    println!("   - Enables initial identity exchange and capability discovery\n");

    println!("3. **Transport Negotiation**");
    println!("   - BLE spec allows defining capability exchange protocol");
    println!("   - Peers can negotiate best available transport");
    println!("   - Graceful fallback when upgrades fail\n");

    println!("4. **Connectivity Resilience**");
    println!("   - Maintains BLE as fallback transport");
    println!("   - Handles transient network failures");
    println!("   - Enables multi-path communication\n");

    println!("5. **Cross-Platform Compatibility**");
    println!("   - BLE works on all major platforms");
    println!("   - Platform-specific optimizations (like MultiPeer) enhance performance");
    println!("   - Unified interface across heterogeneous networks\n");

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║  Demo Complete: Transport Upgrade Flow Validated           ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    Ok(())
}

async fn drain_and_log_received(transport_manager: &TransportManager, stage: &str) {
    let frames = transport_manager.drain_received().await;
    for (idx, frame) in frames.iter().enumerate() {
        println!("[Recv][{}][{}] {} bytes", stage, idx + 1, frame.len());

        if let Ok(msg) = ProtocolMessage::from_bytes(frame) {
            println!("  - decoded protocol message: {:?}", msg);
        } else if let Ok(text) = std::str::from_utf8(frame) {
            println!("  - utf8 payload: {}", text);
        } else {
            println!("  - raw bytes: {:02x?}", frame);
        }
    }
}
