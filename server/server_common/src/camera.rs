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

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum CameraSource {
    #[cfg(feature = "opencv-capture")]
    OpenCV(OpenCVCameraConfig),
    #[cfg(feature = "mediars-capture")]
    MediaRS(MediaRSCameraConfig),
    // TODO other sources could be a camera on an H7 MCU via Ergot...
}

#[derive(Debug, Clone)]
#[cfg(feature = "opencv-capture")]
pub struct OpenCVCameraConfig {
    pub index: i32,
}

#[derive(Debug, Clone)]
#[cfg(feature = "mediars-capture")]
pub struct MediaRSCameraConfig {
    pub device_id: String,
}
