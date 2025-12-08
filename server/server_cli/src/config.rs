use server_common::camera::{CameraDefinition, CameraStreamConfig, CameraSource};
#[cfg(feature = "mediars-capture")]
use server_common::camera::MediaRSCameraConfig;
#[cfg(feature = "opencv-capture")]
use server_common::camera::OpenCVCameraConfig;

// TODO currently hardcoded.  move to config file.
pub fn camera_definitions() -> Vec<CameraDefinition> {
    #[cfg(feature = "development-machine-1")]
    return vec![
        CameraDefinition {
            name: "Microsoft LifeCam Studio".to_string(),
            #[cfg(feature = "opencv-capture")]
            source: CameraSource::OpenCV(OpenCVCameraConfig {
                index: 0,
            }),
            #[cfg(feature = "mediars-capture")]
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
            #[cfg(feature = "opencv-capture")]
            source: CameraSource::OpenCV(OpenCVCameraConfig {
                index: 1,
            }),
            #[cfg(feature = "mediars-capture")]
            source: CameraSource::MediaRS(MediaRSCameraConfig {
                device_id: "\\\\?\\usb#vid_32e6&pid_9211&mi_00#9&351a8e0&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\\global".to_string(),
            }),
            stream_config: CameraStreamConfig {
                jpeg_quality: 95,
            },
            width: 640,
            height: 480,
            fps: 100.0,
            #[cfg(feature = "opencv-capture")]
            four_cc: Some(['Y', 'U', 'Y', '2']),
            #[cfg(feature = "mediars-capture")]
            four_cc: Some(['Y', 'U', 'Y', 'V']),
        },
        CameraDefinition {
            name: "Microsoft XBox Vision Live".to_string(),
            #[cfg(feature = "opencv-capture")]
            source: CameraSource::OpenCV(OpenCVCameraConfig {
                index: 2,
            }),
            #[cfg(feature = "mediars-capture")]
            source: CameraSource::MediaRS(MediaRSCameraConfig {
                device_id: "\\\\?\\usb#vid_045e&pid_0294&mi_00#a&2f495d5d&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\\global".to_string(),
            }),
            stream_config: CameraStreamConfig {
                jpeg_quality: 95,
            },
            width: 640,
            height: 480,
            fps: 30.0,
            #[cfg(feature = "opencv-capture")]
            four_cc: Some(['Y', 'U', 'Y', '2']),
            #[cfg(feature = "mediars-capture")]
            four_cc: Some(['Y', 'U', 'Y', 'V']),
        },
    ];

    #[cfg(feature = "development-machine-2")]
    return vec![
        CameraDefinition {
            name: "Raspberry Pi Global shutter camera".to_string(),
            #[cfg(feature = "opencv-capture")]
            source: CameraSource::OpenCV(OpenCVCameraConfig {
                index: 0,
            }),
            #[cfg(feature = "mediars-capture")]
            source: CameraSource::MediaRS(MediaRSCameraConfig {
                device_id: "/base/axi/pcie@1000120000/rp1/i2c@88000/imx296@1a".to_string(),
            }),
            stream_config: CameraStreamConfig {
                jpeg_quality: 95,
            },
            width: 800,
            height: 600,
            fps: 30.0,
            #[cfg(feature = "opencv-capture")]
            four_cc: Some(['Y', 'U', 'Y', '2']),
            #[cfg(feature = "mediars-capture")]
            four_cc: Some(['Y', 'U', 'Y', 'V']),
        },
        CameraDefinition {
            name: "USB camera 1".to_string(),
            #[cfg(feature = "opencv-capture")]
            source: CameraSource::OpenCV(OpenCVCameraConfig {
                index: 1,
            }),
            #[cfg(feature = "mediars-capture")]
            source: CameraSource::MediaRS(MediaRSCameraConfig {
                device_id: "/base/axi/pcie@1000120000/rp1/usb@200000-1:1.0-2c86:0206".to_string(),
            }),
            stream_config: CameraStreamConfig {
                jpeg_quality: 95,
            },
            width: 640,
            height: 480,
            fps: 30.0,
            #[cfg(feature = "opencv-capture")]
            four_cc: Some(['Y', 'U', 'Y', '2']),
            #[cfg(feature = "mediars-capture")]
            four_cc: Some(['Y', 'U', 'Y', 'V']),
        },
        CameraDefinition {
            name: "USB camera 2".to_string(),
            #[cfg(feature = "opencv-capture")]
            source: CameraSource::OpenCV(OpenCVCameraConfig {
                index: 0,
            }),
            #[cfg(feature = "mediars-capture")]
            source: CameraSource::MediaRS(MediaRSCameraConfig {
                device_id: "/base/axi/pcie@1000120000/rp1/usb@300000-1:1.0-05a3:0144".to_string(),
            }),
            stream_config: CameraStreamConfig {
                jpeg_quality: 95,
            },
            width: 640,
            height: 480,
            fps: 30.0,
            #[cfg(feature = "opencv-capture")]
            four_cc: Some(['Y', 'U', 'Y', '2']),
            #[cfg(feature = "mediars-capture")]
            four_cc: Some(['Y', 'U', 'Y', 'V']),
        },
    ];

    #[cfg(not(any(feature = "development-machine-1", feature = "development-machine-2")))]
    vec![]
}

pub const IO_BOARD_LOCAL_ADDR: &str = "0.0.0.0:8000";
pub const IO_BOARD_REMOTE_ADDR: &str = "192.168.18.64:8000";
pub const OPERATOR_LOCAL_ADDR: &str = "0.0.0.0:8001";
pub const OPERATOR_REMOTE_ADDR: &str = "192.168.18.41:8002";