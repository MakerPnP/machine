use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Schema, Default, Clone, Debug)]
pub struct CameraFrame {
    pub jpeg_bytes: Vec<u8>,
}
