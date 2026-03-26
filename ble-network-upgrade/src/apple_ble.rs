#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;
#[cfg(target_os = "macos")]
use objc2_core_bluetooth::{
    CBAdvertisementDataLocalNameKey, CBAdvertisementDataServiceUUIDsKey, CBPeripheralManager,
    CBUUID,
};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSDictionary, NSString};

#[cfg(target_os = "macos")]
pub struct BleAdvertiser {
    manager: Retained<CBPeripheralManager>,
    _service_uuid: Retained<CBUUID>,
}

#[cfg(target_os = "macos")]
impl BleAdvertiser {
    pub fn start(short_id: [u8; 8]) -> Result<Self, Box<dyn std::error::Error>> {
        let service_uuid_str = NSString::from_str("A83FAF10-9A48-4F55-BC5B-66D91A7C8E11");
        let service_uuid = unsafe { CBUUID::UUIDWithString(&service_uuid_str) };

        let local_name = NSString::from_str(&format!(
            "libp2p-{:02x}{:02x}{:02x}{:02x}",
            short_id[0], short_id[1], short_id[2], short_id[3]
        ));
        let uuids = NSArray::from_retained_slice(&[service_uuid.clone()]);

        let keys = unsafe {
            [
                CBAdvertisementDataLocalNameKey,
                CBAdvertisementDataServiceUUIDsKey,
            ]
        };
        let values: [&AnyObject; 2] = [&*local_name, &*uuids];
        let advertisement = NSDictionary::from_slices(&keys, &values);

        let manager = unsafe { CBPeripheralManager::new() };
        unsafe {
            manager.startAdvertising(Some(&advertisement));
        }

        Ok(Self {
            manager,
            _service_uuid: service_uuid,
        })
    }

    pub fn stop(&self) {
        unsafe {
            self.manager.stopAdvertising();
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub struct BleAdvertiser;

#[cfg(not(target_os = "macos"))]
impl BleAdvertiser {
    pub fn start(_short_id: [u8; 8]) -> Result<Self, Box<dyn std::error::Error>> {
        Err("BLE advertising backend is only available on macOS".into())
    }

    pub fn stop(&self) {}
}
