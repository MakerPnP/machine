use server_common::camera::{CameraDefinition, CameraSource, CameraStreamConfig, MediaRSCameraConfig, OpenCVCameraConfig};

// TODO currently hardcoded.  move to config file.
pub fn camera_definitions() -> Vec<CameraDefinition> {
    vec![
        CameraDefinition {
            name: "Microsoft LifeCam Studio".to_string(),
            // source: CameraSource::OpenCV(OpenCVCameraConfig {
            //     index: 0,
            // }),
            source: CameraSource::MediaRS(MediaRSCameraConfig {
                device_id: "\\\\?\\usb#vid_045e&pid_0772&mi_00#a&6e1307a&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\\global".to_string(),
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
            // source: CameraSource::OpenCV(OpenCVCameraConfig {
            //     index: 1,
            // }),
            source: CameraSource::MediaRS(MediaRSCameraConfig {
                device_id: "\\\\?\\usb#vid_32e6&pid_9211&mi_00#9&351a8e0&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\\global".to_string(),
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
