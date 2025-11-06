
#[derive(Clone)]
pub struct CameraDefinition {
    pub name: String,
    pub source: CameraSource,
    pub stream_config: CameraStreamConfig,

    pub width: u32,
    pub height: u32,
    pub fps: u16,
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
    // TODO currently unused, need to enumerate cameras and pick the right one.
    pub identifier: String,
}
