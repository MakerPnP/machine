#[derive(Clone)]
pub struct CameraDefinition {
    pub name: String,
    pub source: CameraSource,
    pub stream_config: CameraStreamConfig,

    pub width: u32,
    pub height: u32,
    pub fps: f32,
    /// See https://fourcc.org
    pub four_cc: Option<[char; 4]>,
}

#[derive(Clone)]
pub struct CameraStreamConfig {
    pub jpeg_quality: u32,
    // TODO maybe support resizing on the server before sending.
}

#[derive(Clone)]
pub enum CameraSource {
    OpenCV(OpenCVCameraConfig),
    // TODO other sources could be a camera on an H7 MCU via Ergot...
}

#[derive(Clone)]
pub struct OpenCVCameraConfig {
    pub index: i32,
}
