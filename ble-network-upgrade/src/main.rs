use ble_peripheral_rust::gatt::characteristic::Characteristic;
use ble_peripheral_rust::gatt::peripheral_event::PeripheralEvent;
use ble_peripheral_rust::gatt::service::Service;
use ble_peripheral_rust::uuid::ShortUuid;
use ble_peripheral_rust::{Peripheral, PeripheralImpl};
use btleplug::api::bleuuid::BleUuid;
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;
use libp2p::PeerId;
use std::any;
use std::time::Duration;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::channel;
use tokio::sync::{broadcast, oneshot};
use tokio::{signal, time};
use uuid::{uuid, Uuid};

fn generate_identity() -> PeerId {
    let key = libp2p::identity::Keypair::generate_ed25519();
    PeerId::from(key.public())
}

const LP_NAME: &str = "libp2p-node";
const LP_SERVICE_ID: Uuid = uuid!("00001234-0000-1000-8000-00805f9b34fb");
const LP_CHARACTERISTIC_ID: Uuid = uuid!("00005678-0000-1000-8000-00805f9b34fb");
const LOG_DUR: u64 = 2;

async fn broadcast() -> Peripheral {
    let (tx, mut rx) = channel::<PeripheralEvent>(256);
    let mut peripheral = Peripheral::new(tx).await.unwrap();
    while !peripheral.is_powered().await.unwrap() {}
    println!("peripheral started {:?}", peripheral.is_advertising().await);
    peripheral
        .start_advertising(LP_NAME, &[LP_SERVICE_ID])
        .await
        .unwrap();
    peripheral
}

async fn peripheral(p_id: PeerId, close: &mut Receiver<()>) {
    let mut interval = time::interval(Duration::from_secs(LOG_DUR));
    let mut peripheral = broadcast().await;
    peripheral
        .add_service(&Service {
            uuid: LP_SERVICE_ID,
            primary: true,
            characteristics: vec![Characteristic {
                uuid: LP_CHARACTERISTIC_ID,
                value: Some(p_id.to_bytes()),
                ..Default::default()
            }],
        })
        .await
        .unwrap();
    loop {
        tokio::select! {
            _ = close.recv() => {
                println!("Received Ctrl+C, shutting down...");
                break;
            }
            _ = interval.tick() => {
                println!("Peer {} advertising: {:?}", p_id, peripheral.is_advertising().await)
                // scan().await;
            }
        }
    }
}

async fn retrieve_id(
    peripheral: btleplug::platform::Peripheral,
) -> Result<(), Box<dyn std::error::Error>> {
    // connect first
    if let Err(e) = peripheral.connect().await {
        eprintln!("Failed to connect to peripheral: {:?}", e);
        return Err(Box::new(e));
    }
    peripheral.discover_services().await?;
    let chars = peripheral.characteristics();
    let cmd_char = chars.iter().find(|c| c.uuid == LP_CHARACTERISTIC_ID);

    if let Some(characteristic) = cmd_char {
        let data = peripheral.read(characteristic).await?;

        // 5. Convert bytes back to PeerId
        match PeerId::from_bytes(&data) {
            Ok(peer_id) => println!("Successfully discovered PeerID: {}", peer_id),
            Err(e) => eprintln!("Failed to parse PeerID: {:?}", e),
        }
    }
    Ok(())
}

async fn central(close: &mut Receiver<()>) {
    let mut interval = time::interval(Duration::from_secs(LOG_DUR));
    let manager = Manager::new().await.unwrap();
    let adapter = manager.adapters().await.unwrap();
    let central = adapter.iter().nth(0).unwrap();

    central
        .start_scan(ScanFilter {
            services: vec![LP_SERVICE_ID],
        })
        .await
        .unwrap();
    let mut central_events = central.events().await.unwrap();
    loop {
        tokio::select! {
            Some(ev) = central_events.next() => {
                if let CentralEvent::DeviceDiscovered(p_id) | CentralEvent::DeviceUpdated(p_id) = ev {
                    println!("Device discovered: {:?}", p_id);
                    if let Ok(peripheral) = central.peripheral(&p_id).await {
                        if let Some(properties) = peripheral.properties().await.unwrap() {
                            let is_our_node = properties.services.iter().any(|s| s.to_short_string() == LP_SERVICE_ID.to_short_string());
                            if is_our_node {
                                println!("Found our node: {:?}", p_id);
                                println!("-- Node name: {:?}", properties.local_name);
                                println!("-- RSSI: {:?}", properties.rssi);
                                println!("-- Manufacturer data: {:?}", properties.manufacturer_data);
                                println!("connecting...");
                                retrieve_id(peripheral).await;
                            }
                        }
                    }
                }
            }
            _ = interval.tick() => {
                println!("central on");
            }
            _ = close.recv() => break
        }
    }
}

#[tokio::main]
async fn main() {
    // extract out role from args
    let (tx, _rx) = broadcast::channel::<()>(1);
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let role = &args[1];

        if matches!(role.as_str(), "peripheral" | "central") {
            // libp2p identity layer
            let peer_id = generate_identity();
            println!("Generated PeerID: {}", peer_id);

            if role == "peripheral" {
                println!("Running as advertiser");
                peripheral(peer_id, &mut tx.subscribe()).await;
            } else {
                println!("Running as scanner");
                central(&mut tx.subscribe()).await;
            }
            tokio::select! {
               _ = signal::ctrl_c() => {
                   println!("Received Ctrl+C, shutting down...");
                   tx.send(()).unwrap();
               }
            }
        }
        eprintln!("Invalid role specified {}", role);
    } else {
        eprintln!("Need a role - peripheral or central")
    }
}
