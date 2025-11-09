use alloc::string::String;

use ergot::traits::Schema;
use serde::{Deserialize, Serialize};

use crate::camera::{CameraCommand, CameraCommandError, CameraIdentifier, CameraStreamerCommandResult};

// TODO determine which is better: a) a single enum for all commands, or b) maintain many specific-endpoints?
#[derive(Schema, Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum OperatorCommandRequest {
    Heartbeat(u64),
    CameraCommand(CameraIdentifier, CameraCommand),
}

#[derive(Debug, Serialize, Deserialize, Schema, Clone)]
pub enum OperatorCommandResponse {
    Acknowledged,
    CameraCommandResult(Result<CameraStreamerCommandResult, CameraCommandError>),
}

#[derive(Debug, Serialize, Deserialize, Schema, Clone, PartialEq, Eq)]
pub enum CommandArg {
    String(String),
    I32(i32),
    U32(u32),
}
