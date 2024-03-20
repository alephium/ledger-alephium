use ledger_device_sdk::nvm::{AtomicStorage, SingleStorage};
use ledger_device_sdk::NVMData;

#[link_section = ".nvm_data"]
static mut BLIND_SIGNING: NVMData<AtomicStorage<u8>> = NVMData::new(AtomicStorage::new(&0));

pub fn is_blind_signing_enabled() -> bool {
    let blind_signing = unsafe { BLIND_SIGNING.get_mut() };
    let value = *blind_signing.get_ref();
    value == 1
}

pub fn update_blind_signing() {
    let blind_signing = unsafe { BLIND_SIGNING.get_mut() };
    if *blind_signing.get_ref() == 1 {
        blind_signing.update(&0);
    } else {
        blind_signing.update(&1);
    }
}
