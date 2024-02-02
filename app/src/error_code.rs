use ledger_device_sdk::io::Reply;
use ledger_device_sdk::io::StatusWords;

#[derive(Debug)]
#[repr(u16)]
pub enum ErrorCode {
    Ok = StatusWords::Ok as u16,
    BadCla = StatusWords::BadCla as u16,
    BadIns = StatusWords::BadIns as u16,
    BadP1P2 = StatusWords::BadP1P2 as u16,
    BadLen = StatusWords::BadLen as u16,
    UserCancelled = StatusWords::UserCancelled as u16,
    TxDecodeFail = 0xE000,
    TxSignFail = 0xE001,
    Overflow = 0xE002,
    DerivePathDecodeFail = 0xE003,
    BlindSigningNotEnabled = 0xE004,
    InternalError = 0xEF00,
}

impl From<ErrorCode> for Reply {
    fn from(sw: ErrorCode) -> Reply {
        Reply(sw as u16)
    }
}
