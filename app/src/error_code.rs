use ledger_device_sdk::io::Reply;
use ledger_device_sdk::io::StatusWords;

#[repr(u16)]
pub enum ErrorCode {
  Ok = StatusWords::Ok as u16,
  BadCla = StatusWords::BadCla as u16,
  BadIns = StatusWords::BadIns as u16,
  BadP1P2 = StatusWords::BadP1P2 as u16,
  BadLen = StatusWords::BadLen as u16,
  UserCancelled = StatusWords::UserCancelled as u16,
  TxDecodeFail = 0xF000,
  TxSignFail = 0xF001,
  InvalidHashLength = 0xF002,
  InvalidParameter = 0xF003,
  InternalError = 0xFF00,
  Unknown = StatusWords::Unknown as u16,
}

impl From<ErrorCode> for Reply {
  fn from(sw: ErrorCode) -> Reply {
    Reply(sw as u16)
  }
}
