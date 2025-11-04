use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Schema, Default, Clone, Debug)]
pub struct CameraFrame {
    pub frame_number: u64,
    pub jpeg_bytes: Vec<u8>,
}
