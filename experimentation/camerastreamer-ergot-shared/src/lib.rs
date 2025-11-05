use std::ops::{Deref, DerefMut};
use postcard_schema::Schema;
use postcard_schema::schema::{DataModelType, NamedType};
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimeStampUTC(chrono::DateTime<chrono::Utc>);

impl postcard_schema::Schema for TimeStampUTC {
    const SCHEMA: &'static NamedType = &NamedType { name: "timestamp_utc", ty: &DataModelType::I64 };
}

impl Deref for TimeStampUTC {
    type Target = chrono::DateTime<chrono::Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TimeStampUTC {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<chrono::DateTime<chrono::Utc>> for TimeStampUTC {
    fn from(dt: chrono::DateTime<chrono::Utc>) -> Self {
        Self(dt)
    }
}