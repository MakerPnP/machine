use alloc::vec::Vec;
use core::fmt::Display;
use core::ops::Deref;

use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

use crate::commands::CommandArg;
use crate::common::TimeStampUTC;

#[derive(Serialize, Deserialize, Schema, Clone, Debug)]
pub struct CameraFrameChunk {
    pub frame_number: u64,
    pub kind: CameraFrameChunkKind,
}

#[derive(Serialize, Deserialize, Schema, Clone, Debug)]
pub enum CameraFrameChunkKind {
    Meta(CameraFrameMeta),
    ImageChunk(CameraFrameImageChunk),
}

#[derive(Serialize, Deserialize, Schema, Clone, Debug)]
pub struct CameraFrameMeta {
    pub total_chunks: u32,
    pub frame_timestamp: TimeStampUTC,
    pub total_bytes: u32,
}

#[derive(Serialize, Deserialize, Schema, Clone, Debug)]
pub struct CameraFrameImageChunk {
    pub chunk_index: u32,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Schema, Clone, PartialEq)]
pub enum CameraCommand {
    StartStreaming { port_id: u8, fps: f32 },
    StopStreaming { port_id: u8 },
    // TODO
    // GetCameraProperties,
    // SetCameraProperties { properties: CameraProperties },
}

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Schema,
    Clone,
    Copy,
    PartialEq,
    Hash,
    Eq,
    PartialOrd,
    Ord
)]
pub struct CameraIdentifier(u8);

impl CameraIdentifier {
    pub fn new(id: u8) -> Self {
        Self(id)
    }
}

impl Display for CameraIdentifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "C{:03}", self.0)
    }
}
impl From<CameraIdentifier> for u8 {
    fn from(value: CameraIdentifier) -> Self {
        value.0
    }
}

impl Deref for CameraIdentifier {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize, Schema, Clone)]
pub enum CameraStreamerCommandResult {
    Acknowledged,
    // TODO
    // CameraProperties { properties: CameraProperties },
}

#[derive(Debug, Serialize, Deserialize, Schema, Clone)]
pub struct CameraCommandError {
    pub code: CameraCommandErrorCode,
    pub args: Vec<CommandArg>,
}

#[derive(Debug, Serialize, Deserialize, Schema, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CameraCommandErrorCode {
    InvalidIdentifier = 0,
    Busy = 1,
    NotStreaming = 2,
}

impl CameraCommandError {
    pub fn new(code: CameraCommandErrorCode) -> Self {
        Self {
            code,
            args: Vec::new(),
        }
    }

    pub fn with_args(mut self, args: Vec<CommandArg>) -> Self {
        self.args = args;
        self
    }
}
