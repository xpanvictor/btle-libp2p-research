use ble_peripheral_rust::gatt::peripheral_event::PeripheralEvent;
use ble_peripheral_rust::uuid::ShortUuid;
use ble_peripheral_rust::{Peripheral, PeripheralImpl};
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;
use std::any;
use std::time::Duration;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::channel;
use tokio::sync::{broadcast, oneshot};
use tokio::{signal, time};
use uuid::{uuid, Uuid};

fn generate_advertisement_data(peer_id: &str) -> &str {
    peer_id
}

const LP_NAME: &str = "libp2p-node";
const LP_SERVICE_ID: Uuid = uuid!("00001234-0000-1000-8000-00805f9b34fb");
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

async fn find_node(central: &Adapter) -> Option<btleplug::platform::Peripheral> {
    let peripherals = central.peripherals().await.unwrap();
    for peripheral in peripherals {
        let properties = peripheral.properties().await.unwrap();
        if let Some(properties) = properties {
            if properties.local_name == Some(LP_NAME.to_string()) {
                return Some(peripheral);
            }
        }
    }
    None
}

async fn peripheral(close: &mut Receiver<()>) {
    let mut interval = time::interval(Duration::from_secs(LOG_DUR));
    let mut peripheral = broadcast().await;
    loop {
        tokio::select! {
            _ = close.recv() => {
                println!("Received Ctrl+C, shutting down...");
                break;
            }
            _ = interval.tick() => {
                println!("peripheral advertising: {:?}", peripheral.is_advertising().await);
                // scan().await;
            }
        }
    }
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
                if let CentralEvent::DeviceDiscovered(p_id) = ev {
                    println!("Device discovered: {:?}", p_id);
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
            if role == "peripheral" {
                println!("Running as advertiser");
                peripheral(&mut tx.subscribe()).await;
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
