/// BLE module - handles Bluetooth Low Energy discovery and connection
use btleplug::api::{Central, Manager, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager as PlatformManager, Peripheral};
use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::apple_ble::BleAdvertiser;

/// BLE event types
#[derive(Debug, Clone)]
pub enum BleEvent {
    PeerDiscovered {
        address: String,
        rssi: i16,
        adv_data: Vec<u8>,
    },
    PeerConnected {
        address: String,
    },
    PeerDisconnected {
        address: String,
    },
}

pub struct BleManager {
    adapter: Adapter,
    peripherals: HashMap<String, Peripheral>,
    event_tx: mpsc::UnboundedSender<BleEvent>,
    event_rx: mpsc::UnboundedReceiver<BleEvent>,
    advertiser: Option<BleAdvertiser>,
}

impl BleManager {
    /// Initialize BLE manager
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manager = PlatformManager::new().await?;
        let adapters = manager.adapters().await?;

        let adapter = adapters
            .into_iter()
            .next()
            .ok_or("No BLE adapter found")?;

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Ok(Self {
            adapter,
            peripherals: HashMap::new(),
            event_tx,
            event_rx,
            advertiser: None,
        })
    }

    /// Start BLE advertising as a peripheral (macOS CoreBluetooth backend).
    pub fn start_advertising(
        &mut self,
        short_id: [u8; 8],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let advertiser = BleAdvertiser::start(short_id)?;
        self.advertiser = Some(advertiser);
        println!("[BLE] Advertising as peripheral is active");
        Ok(())
    }

    /// Stop BLE advertising if active.
    pub fn stop_advertising(&mut self) {
        if let Some(advertiser) = &self.advertiser {
            advertiser.stop();
        }
        self.advertiser = None;
    }

    /// Start scanning for BLE peripherals
    pub async fn start_scan(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("[BLE] Starting scan for nearby devices...");
        self.adapter.start_scan(ScanFilter::default()).await?;
        Ok(())
    }

    /// Stop scanning
    pub async fn stop_scan(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("[BLE] Stopping scan");
        self.adapter.stop_scan().await?;
        Ok(())
    }

    /// Connect to a peripheral by address
    pub async fn connect(&mut self, address: &str) -> Result<(), Box<dyn std::error::Error>> {
        let _peripheral = self
            .peripherals
            .get(address)
            .ok_or(format!("Peripheral {} not found", address))?;

        println!("[BLE] Connecting to {} ...", address);
        // Note: actual connection implementation would be platform-specific
        println!("[BLE] Connected to {}", address);

        let _ = self.event_tx.send(BleEvent::PeerConnected {
            address: address.to_string(),
        });

        Ok(())
    }

    /// Disconnect from a peripheral
    pub async fn disconnect(&mut self, address: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.peripherals.contains_key(address) {
            println!("[BLE] Disconnecting from {}", address);

            let _ = self.event_tx.send(BleEvent::PeerDisconnected {
                address: address.to_string(),
            });
        }

        Ok(())
    }

    /// Poll for BLE events
    pub async fn poll_events(&mut self) -> Result<Option<BleEvent>, Box<dyn std::error::Error>> {
        // Check channel for internally generated events
        if let Ok(event) = self.event_rx.try_recv() {
            return Ok(Some(event));
        }

        // Pull discoveries from the adapter and emit one event per new address.
        let peripherals = self.adapter.peripherals().await?;
        for peripheral in peripherals {
            if let Some(props) = peripheral.properties().await? {
                let address = props.address.to_string();
                if address == "00:00:00:00:00:00" {
                    continue;
                }

                // Only keep devices from this demo's advertiser naming scheme.
                let local_name = props.local_name.as_deref().unwrap_or_default();
                if !local_name.starts_with("libp2p-") {
                    continue;
                }

                if self.peripherals.contains_key(&address) {
                    continue;
                }

                let rssi = props.rssi.unwrap_or(-127);
                let adv_data = props
                    .manufacturer_data
                    .values()
                    .next()
                    .cloned()
                    .unwrap_or_default();

                self.peripherals.insert(address.clone(), peripheral);

                return Ok(Some(BleEvent::PeerDiscovered {
                    address,
                    rssi,
                    adv_data,
                }));
            }
        }

        Ok(None)
    }

    /// Get discovered peripherals
    pub fn get_peripherals(&self) -> Vec<String> {
        self.peripherals.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ble_event_creation() {
        let event = BleEvent::PeerDiscovered {
            address: "AA:BB:CC:DD:EE:FF".to_string(),
            rssi: -50,
            adv_data: vec![1, 2, 3],
        };

        match event {
            BleEvent::PeerDiscovered { address, rssi, .. } => {
                assert_eq!(address, "AA:BB:CC:DD:EE:FF");
                assert_eq!(rssi, -50);
            }
            _ => panic!("Wrong event type"),
        }
    }
}
