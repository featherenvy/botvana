use serde::{Deserialize, Serialize};

use super::msg::Message;

#[derive(Deserialize, Serialize, Debug)]
pub struct Frame {
    version: u8,
    size: u32,
    value: Message,
}
