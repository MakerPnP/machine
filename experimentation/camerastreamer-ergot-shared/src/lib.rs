use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Schema, Default, Clone, Debug)]
pub struct CameraFrame {
    pub frame_number: u64,
    pub jpeg_bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize, Schema, Default, Clone, Debug)]
pub struct CameraFrameChunk {
    pub frame_number: u64,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub total_bytes: u32,
    pub bytes: Vec<u8>,
}
