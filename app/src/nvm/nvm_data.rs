pub struct NVMData<T> {
    data: T,
}

impl<T> NVMData<T> {
    pub const fn new(data: T) -> NVMData<T> {
        NVMData { data }
    }

    pub fn get_mut(&mut self) -> &mut T {
        ledger_secure_sdk_sys::pic_rs_mut(&mut self.data)
    }

    pub fn get_ref(&self) -> &T {
        ledger_secure_sdk_sys::pic_rs(&self.data)
    }
}
