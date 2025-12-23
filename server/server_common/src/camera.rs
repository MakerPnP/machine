#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct CameraDefinition {
    pub name: String,
    /// Only one source is used, see feature flags.
    pub sources: Vec<CameraSource>,
    pub stream_config: CameraStreamConfig,

    pub width: u32,
    pub height: u32,
    pub fps: f32,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct CameraStreamConfig {
    /// 0 - 100, 100 is highest quality
    /// Note: lower quality = less data = less network traffic and server/client load = higher fps when server system is IO or CPU bound
    ///       70 is a good starting point.
    ///       image quality only affects the stream and NOT the CV pipeline.
    pub jpeg_quality: u8,
    // TODO maybe support resizing on the server before sending.
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[non_exhaustive]
pub enum CameraSource {
    OpenCV(OpenCVCameraConfig),
    MediaRS(MediaRSCameraConfig),
    // TODO other sources could be a camera on an H7 MCU via Ergot...
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct OpenCVCameraConfig {
    pub index: i32,
    /// See https://fourcc.org
    pub four_cc: Option<[char; 4]>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MediaRSCameraConfig {
    pub device_id: String,
    /// See https://fourcc.org
    pub four_cc: Option<[char; 4]>,
}
