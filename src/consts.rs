pub const DEFAULT_NETWORK_TIMEOUT_MS: u64 = 10000_u64;
pub const DEFAULT_BACKOFF_BASE_MS: u64 = 100_u64;

pub(crate) const ERROR_ILLEGAL_DATA_VALUE: &str = "Modbus function 3: Illegal data value";
pub(crate) const ERROR_GATEWAY_DEVICE_FAILED_TO_RESPOND: &str =
    "Modbus function 3: Gateway target device failed to respond";
pub(crate) const ERROR_INVALID_RESPONSE_HEADER: &str = "Invalid response header: expected/request";
