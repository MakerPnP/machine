//! The status of the cameras, from the configured cameras, the detected cameras and the cameras being streamed.

use std::collections::HashSet;

use operator_shared::camera::CameraIdentifier;
use server_common::camera::{CameraDefinition, CameraSource, DetectedCamera};

#[derive(Debug, Clone, PartialEq)]
pub struct CameraStatus {
    /// `None` for a detected camera that isn't configured.
    pub configured: Option<ConfiguredCamera>,
    /// The source that is, or would be, used.  `None` when a configured camera has no source supported by the enabled
    /// capture apis.
    pub source: Option<CameraSourceStatus>,
    pub state: CameraState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfiguredCamera {
    pub identifier: CameraIdentifier,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CameraSourceStatus {
    OpenCV {
        index: i32,
    },
    MediaRS {
        device_id: String,
        /// The device's name, when detected.
        name: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CameraState {
    /// Detected, but not configured.
    New,
    /// Configured, detected and idle.
    Matched,
    /// Configured, but not detected.
    Missing,
    /// Being streamed.
    Streaming,
}

/// The capture apis enabled by the crate features.
#[derive(Debug, Clone, Copy)]
pub struct CaptureApis {
    pub opencv: bool,
    pub mediars: bool,
}

impl CaptureApis {
    pub const ENABLED: Self = Self {
        opencv: cfg!(feature = "opencv-capture"),
        mediars: cfg!(feature = "mediars-capture"),
    };

    fn supports(&self, source: &CameraSource) -> bool {
        match source {
            CameraSource::OpenCV(_) => self.opencv,
            CameraSource::MediaRS(_) => self.mediars,
            _ => false,
        }
    }
}

/// Whether `source` refers to the detected `camera`.
fn refers_to(source: &CameraSource, camera: &DetectedCamera) -> bool {
    match (source, camera) {
        (CameraSource::OpenCV(config), DetectedCamera::OpenCV { index }) => config.index == *index,
        (CameraSource::MediaRS(config), DetectedCamera::MediaRS { device_id, .. }) => config.device_id == *device_id,
        _ => false,
    }
}

/// The configured cameras, in config order, followed by the detected cameras that aren't configured.
///
/// Like the capture loop, a configured camera uses the first of its sources that is supported and detected.
pub fn camera_statuses(
    definitions: &[CameraDefinition],
    detected: &[DetectedCamera],
    streaming: &HashSet<CameraIdentifier>,
    apis: CaptureApis,
) -> Vec<CameraStatus> {
    let detected_camera = |source: &CameraSource| {
        detected
            .iter()
            .find(|camera| refers_to(source, camera))
    };

    let configured = definitions
        .iter()
        .enumerate()
        .map(|(index, definition)| {
            let identifier = CameraIdentifier::new(index as u8);

            let source_status = |source: &CameraSource| match source {
                CameraSource::OpenCV(config) => Some(CameraSourceStatus::OpenCV { index: config.index }),
                CameraSource::MediaRS(config) => Some(CameraSourceStatus::MediaRS {
                    device_id: config.device_id.clone(),
                    name: match detected_camera(source) {
                        Some(DetectedCamera::MediaRS { name, .. }) => Some(name.clone()),
                        _ => None,
                    },
                }),
                _ => None,
            };

            let mut supported = definition
                .sources
                .iter()
                .filter(|source| apis.supports(source));
            let first_supported = supported.clone().next();
            let present = supported.find(|source| detected_camera(source).is_some());

            let state = if streaming.contains(&identifier) {
                CameraState::Streaming
            } else if present.is_some() {
                CameraState::Matched
            } else {
                CameraState::Missing
            };

            CameraStatus {
                configured: Some(ConfiguredCamera {
                    identifier,
                    name: definition.name.clone(),
                    width: definition.width,
                    height: definition.height,
                    fps: definition.fps,
                }),
                source: present
                    .or(first_supported)
                    .and_then(source_status),
                state,
            }
        });

    let is_configured = |camera: &DetectedCamera| {
        definitions
            .iter()
            .flat_map(|definition| definition.sources.iter())
            .any(|source| refers_to(source, camera))
    };
    let new = detected
        .iter()
        .filter(|camera| !is_configured(camera))
        .filter_map(|camera| {
            let source = match camera {
                DetectedCamera::OpenCV { index } => CameraSourceStatus::OpenCV { index: *index },
                DetectedCamera::MediaRS { device_id, name } => CameraSourceStatus::MediaRS {
                    device_id: device_id.clone(),
                    name: Some(name.clone()),
                },
                _ => return None,
            };
            Some(CameraStatus {
                configured: None,
                source: Some(source),
                state: CameraState::New,
            })
        });

    configured.chain(new).collect()
}

#[cfg(test)]
mod tests {
    use server_common::camera::{CameraStreamConfig, MediaRSCameraConfig, OpenCVCameraConfig};

    use super::*;

    const OPENCV: CaptureApis = CaptureApis {
        opencv: true,
        mediars: false,
    };
    const MEDIARS: CaptureApis = CaptureApis {
        opencv: false,
        mediars: true,
    };

    fn opencv(index: i32) -> CameraSource {
        CameraSource::OpenCV(OpenCVCameraConfig { index, four_cc: None })
    }

    fn mediars(device_id: &str) -> CameraSource {
        CameraSource::MediaRS(MediaRSCameraConfig {
            device_id: device_id.to_string(),
            four_cc: None,
        })
    }

    fn definition(name: &str, sources: Vec<CameraSource>) -> CameraDefinition {
        CameraDefinition {
            name: name.to_string(),
            sources,
            stream_config: CameraStreamConfig { jpeg_quality: 95 },
            width: 640,
            height: 480,
            fps: 30.0,
        }
    }

    fn detected(device_id: &str) -> DetectedCamera {
        DetectedCamera::MediaRS {
            device_id: device_id.to_string(),
            name: format!("{} name", device_id),
        }
    }

    fn states(statuses: &[CameraStatus]) -> Vec<CameraState> {
        statuses
            .iter()
            .map(|status| status.state)
            .collect()
    }

    #[test]
    fn mediars_cameras_are_matched_missing_or_new() {
        let definitions = [
            definition("a", vec![mediars("dev-a")]),
            definition("b", vec![mediars("dev-b")]),
        ];
        let statuses = camera_statuses(
            &definitions,
            &[detected("dev-a"), detected("dev-c")],
            &HashSet::new(),
            MEDIARS,
        );

        use CameraState::*;
        assert_eq!(states(&statuses), [Matched, Missing, New]);
        assert_eq!(
            statuses[0].source,
            Some(CameraSourceStatus::MediaRS {
                device_id: "dev-a".to_string(),
                name: Some("dev-a name".to_string())
            })
        );
        assert_eq!(
            statuses[1].source,
            Some(CameraSourceStatus::MediaRS {
                device_id: "dev-b".to_string(),
                name: None
            })
        );
        assert_eq!(statuses[2].configured, None);
        assert_eq!(
            statuses[0]
                .configured
                .as_ref()
                .unwrap()
                .name,
            "a"
        );
    }

    #[test]
    fn opencv_cameras_are_matched_or_missing() {
        let definitions = [
            definition("a", vec![opencv(0), mediars("dev-a")]),
            definition("b", vec![opencv(1)]),
        ];
        let statuses = camera_statuses(
            &definitions,
            &[DetectedCamera::OpenCV { index: 0 }],
            &HashSet::new(),
            OPENCV,
        );
        assert_eq!(states(&statuses), [CameraState::Matched, CameraState::Missing]);
        assert_eq!(statuses[0].source, Some(CameraSourceStatus::OpenCV { index: 0 }));
        assert_eq!(statuses[1].source, Some(CameraSourceStatus::OpenCV { index: 1 }));
    }

    #[test]
    fn unsupported_sources_are_ignored() {
        let definitions = [definition("a", vec![opencv(1)])];
        let statuses = camera_statuses(&definitions, &[], &HashSet::new(), MEDIARS);
        assert_eq!(states(&statuses), [CameraState::Missing]);
        assert_eq!(statuses[0].source, None);
    }

    #[test]
    fn streaming() {
        let definitions = [
            definition("a", vec![mediars("dev-a")]),
            definition("b", vec![mediars("dev-b")]),
        ];
        let streaming = HashSet::from([CameraIdentifier::new(1)]);
        let statuses = camera_statuses(
            &definitions,
            &[detected("dev-a"), detected("dev-b")],
            &streaming,
            MEDIARS,
        );
        assert_eq!(states(&statuses), [CameraState::Matched, CameraState::Streaming]);
    }
}
