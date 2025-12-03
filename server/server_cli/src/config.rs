use server_common::camera::{CameraDefinition, CameraSource, CameraStreamConfig, OpenCVCameraConfig};

// TODO currently hardcoded.  move to config file.
pub fn camera_definitions() -> Vec<CameraDefinition> {
    vec![
        CameraDefinition {
            name: "Microsoft LifeCam Studio".to_string(),
            source: CameraSource::OpenCV(OpenCVCameraConfig {
                index: 0,
            }),
            stream_config: CameraStreamConfig {
                jpeg_quality: 95,
            },
            width: 1920,
            height: 1280,
            fps: 30.0,
            four_cc: None,
        },
        CameraDefinition {
            name: "B&W Global shutter".to_string(),
            source: CameraSource::OpenCV(OpenCVCameraConfig {
                index: 1,
            }),
            stream_config: CameraStreamConfig {
                jpeg_quality: 95,
            },
            width: 640,
            height: 480,
            fps: 100.0,
            four_cc: Some(['Y', 'U', 'Y', '2']),
        },
        CameraDefinition {
            name: "Microsoft XBox Vision Live".to_string(),
            source: CameraSource::OpenCV(OpenCVCameraConfig {
                index: 2,
            }),
            stream_config: CameraStreamConfig {
                jpeg_quality: 95,
            },
            width: 640,
            height: 480,
            fps: 30.0,
            four_cc: Some(['Y', 'U', 'Y', '2']),
        },
    ]
}
