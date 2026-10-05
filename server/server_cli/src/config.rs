use ioboard_shared::discovery::{ParseSerialNumberError, SerialNumber};
#[cfg(feature = "mediars-capture")]
use server_common::camera::MediaRSCameraConfig;
#[cfg(feature = "opencv-capture")]
use server_common::camera::OpenCVCameraConfig;
use server_common::camera::{CameraDefinition, CameraSource, CameraStreamConfig};

// TODO currently hardcoded.  move to config file.
pub fn camera_definitions() -> Vec<CameraDefinition> {
    #[cfg(feature = "development-machine-1")]
    return vec![
        CameraDefinition {
            name: "Microsoft LifeCam Studio".to_string(),
            sources: vec![
                #[cfg(feature = "opencv-capture")]
                CameraSource::OpenCV(OpenCVCameraConfig {
                    index: 0,
                    four_cc: None,
                }),
                #[cfg(feature = "mediars-capture")]
                CameraSource::MediaRS(MediaRSCameraConfig {
                    device_id: "\\\\?\\usb#vid_045e&pid_0772&mi_00#c&2c5491c4&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\\global".to_string(),
                    four_cc: None,
                }),
            ],
            stream_config: CameraStreamConfig {
                jpeg_quality: 95,
            },
            width: 1920,
            height: 1280,
            fps: 30.0,
        },
        CameraDefinition {
            name: "B&W Global shutter".to_string(),
            sources: vec![
                #[cfg(feature = "opencv-capture")]
                CameraSource::OpenCV(OpenCVCameraConfig {
                    index: 1,
                    four_cc: Some(['Y', 'U', 'Y', '2']),
                }),
                #[cfg(feature = "mediars-capture")]
                CameraSource::MediaRS(MediaRSCameraConfig {
                    device_id: "\\\\?\\usb#vid_32e6&pid_9211&mi_00#c&2393f5c7&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\\global".to_string(),
                    four_cc: Some(['Y', 'U', 'Y', 'V']),
                }),
            ],
            stream_config: CameraStreamConfig {
                jpeg_quality: 95,
            },
            width: 640,
            height: 480,
            fps: 100.0,
        },
        // CameraDefinition {
        //     name: "Microsoft XBox Vision Live".to_string(),
        //     sources: vec![
        //         #[cfg(feature = "opencv-capture")]
        //         CameraSource::OpenCV(OpenCVCameraConfig {
        //             index: 2,
        //             four_cc: Some(['Y', 'U', 'Y', '2']),
        //         }),
        //         #[cfg(feature = "mediars-capture")]
        //         CameraSource::MediaRS(MediaRSCameraConfig {
        //             device_id: "\\\\?\\usb#vid_045e&pid_0294&mi_00#a&2f495d5d&0&0000#{e5323777-f976-4f5b-9b55-b94699c46e44}\\global".to_string(),
        //             four_cc: Some(['Y', 'U', 'Y', 'V']),
        //         }),
        //     ],
        //     stream_config: CameraStreamConfig {
        //         jpeg_quality: 95,
        //     },
        //     width: 640,
        //     height: 480,
        //     fps: 30.0,
        // },
    ];

    #[cfg(feature = "development-machine-2")]
    return vec![
        CameraDefinition {
            name: "Raspberry Pi Global shutter camera".to_string(),
            sources: vec![
                #[cfg(feature = "opencv-capture")]
                CameraSource::OpenCV(OpenCVCameraConfig {
                    index: 0,
                    four_cc: Some(['Y', 'U', 'Y', '2']),
                }),
                #[cfg(feature = "mediars-capture")]
                CameraSource::MediaRS(MediaRSCameraConfig {
                    device_id: "/base/axi/pcie@1000120000/rp1/i2c@88000/imx296@1a".to_string(),
                    four_cc: Some(['Y', 'U', 'Y', 'V']),
                }),
            ],
            stream_config: CameraStreamConfig {
                jpeg_quality: 95,
            },
            width: 800,
            height: 600,
            fps: 30.0,
        },
        CameraDefinition {
            name: "USB camera 1".to_string(),
            sources: vec![
                #[cfg(feature = "opencv-capture")]
                CameraSource::OpenCV(OpenCVCameraConfig {
                    index: 1,
                    four_cc: Some(['Y', 'U', 'Y', '2']),
                }),
                #[cfg(feature = "mediars-capture")]
                CameraSource::MediaRS(MediaRSCameraConfig {
                    device_id: "/base/axi/pcie@1000120000/rp1/usb@200000-1:1.0-2c86:0206".to_string(),
                    four_cc: Some(['Y', 'U', 'Y', 'V']),
                }),
            ],
            stream_config: CameraStreamConfig {
                jpeg_quality: 95,
            },
            width: 640,
            height: 480,
            fps: 30.0,
        },
        CameraDefinition {
            name: "USB camera 2".to_string(),
            sources: vec![
                #[cfg(feature = "opencv-capture")]
                CameraSource::OpenCV(OpenCVCameraConfig {
                    index: 0,
                    four_cc: Some(['Y', 'U', 'Y', '2']),
                }),
                #[cfg(feature = "mediars-capture")]
                CameraSource::MediaRS(MediaRSCameraConfig {
                    device_id: "/base/axi/pcie@1000120000/rp1/usb@300000-1:1.0-05a3:0144".to_string(),
                    four_cc: Some(['Y', 'U', 'Y', 'V']),
                }),
            ],
            stream_config: CameraStreamConfig {
                jpeg_quality: 95,
            },
            width: 640,
            height: 480,
            fps: 30.0,
        },
    ];

    #[cfg(not(any(feature = "development-machine-1", feature = "development-machine-2")))]
    vec![]
}

/// The operator UI connects to this port, the server learns the operator UI's address from the first packet it sends.
pub const OPERATOR_LOCAL_ADDR: &str = "0.0.0.0:8001";

/// Each io board definition uses a fixed local UDP port, this port + the index of the definition.
///
/// The port must not change between server restarts since io boards stay claimed by the server endpoint that first
/// claimed them, until the io board is reset.
pub const IO_BOARD_LOCAL_PORT_BASE: u16 = 8200;

// Rules:
// 1) The names in config structures should be as simple as possible.
// 2) Define them in a way to mitigate or minimize having to migrate them from one version to another.

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub cameras: Vec<CameraDefinition>,
    /// Each definition is one physical io board.
    pub io_boards: Vec<IoBoardDefinition>,
}

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut ids = Vec::new();
        let mut names = Vec::new();
        for definition in &self.io_boards {
            if definition.name.trim().is_empty() {
                return Err(ConfigError::EmptyIoBoardName);
            }
            if names.contains(&&definition.name) {
                return Err(ConfigError::DuplicateIoBoardName(definition.name.clone()));
            }
            names.push(&definition.name);

            match &definition.connection {
                ConnectionKind::Discovered(DiscoveredIoBoard::Id(id)) => {
                    if ids.contains(id) {
                        return Err(ConfigError::DuplicateIoBoardId(*id));
                    }
                    ids.push(*id);
                }
                ConnectionKind::Discovered(DiscoveredIoBoard::First) => {}
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    EmptyIoBoardName,
    DuplicateIoBoardName(String),
    DuplicateIoBoardId(IoBoardId),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::EmptyIoBoardName => write!(f, "io board name must not be empty"),
            ConfigError::DuplicateIoBoardName(name) => write!(
                f,
                "io board name is used by more than one io board definition. name: {}",
                name
            ),
            ConfigError::DuplicateIoBoardId(id) => write!(
                f,
                "io board id is used by more than one io board definition. id: {}",
                id.0
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct IoBoardDefinition {
    /// Unique, used to refer to the io board, e.g. when assigning physical io.
    pub name: String,
    pub connection: ConnectionKind,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[non_exhaustive]
pub enum ConnectionKind {
    /// An io board that advertises itself on the local network, connected to when it's discovered.
    Discovered(DiscoveredIoBoard),
    // FUTURE: USB, RS485, etc.
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum DiscoveredIoBoard {
    /// The first discovered io board that doesn't match the `Id` of any other io board definition.
    ///
    /// Since boards are assigned in the order they are discovered, use this only when there is a single io board on the
    /// network, otherwise physical io may be assigned to the wrong board.
    First,
    /// The io board with this serial number.
    Id(IoBoardId),
}

/// An io board serial number, as 24 hex digits, e.g. "2C0024000851333234363838".
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct IoBoardId(pub SerialNumber);

impl TryFrom<String> for IoBoardId {
    type Error = ParseSerialNumberError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse().map(Self)
    }
}

impl From<IoBoardId> for String {
    fn from(value: IoBoardId) -> Self {
        value.0.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_io_boards(content: &str) -> Result<Vec<IoBoardDefinition>, ron::error::SpannedError> {
        ron::from_str(content)
    }

    #[test]
    fn io_boards() {
        let io_boards = parse_io_boards(
            r#"[
                IoBoardDefinition(name: "head", connection: Discovered(First)),
                IoBoardDefinition(name: "gantry", connection: Discovered(Id("2C0024000851333234363838"))),
            ]"#,
        )
        .unwrap();

        assert_eq!(io_boards[0].name, "head");
        assert_eq!(io_boards[1].name, "gantry");
        assert!(matches!(
            io_boards[0].connection,
            ConnectionKind::Discovered(DiscoveredIoBoard::First)
        ));
        let ConnectionKind::Discovered(DiscoveredIoBoard::Id(id)) = io_boards[1].connection else {
            panic!()
        };
        assert_eq!(
            id,
            IoBoardId(
                "2C0024000851333234363838"
                    .parse()
                    .unwrap()
            )
        );

        assert!(
            parse_io_boards(r#"[IoBoardDefinition(name: "head", connection: Discovered(Id("not-a-serial-number")))]"#)
                .is_err()
        );
    }

    #[test]
    fn duplicate_io_board_ids_are_invalid() {
        let config = Config {
            cameras: vec![],
            io_boards: parse_io_boards(
                r#"[
                    IoBoardDefinition(name: "head", connection: Discovered(Id("2C0024000851333234363838"))),
                    IoBoardDefinition(name: "gantry", connection: Discovered(Id("2C0024000851333234363838"))),
                ]"#,
            )
            .unwrap(),
        };
        assert!(matches!(config.validate(), Err(ConfigError::DuplicateIoBoardId(_))));
    }

    #[test]
    fn duplicate_or_empty_io_board_names_are_invalid() {
        let config = Config {
            cameras: vec![],
            io_boards: parse_io_boards(
                r#"[
                    IoBoardDefinition(name: "head", connection: Discovered(First)),
                    IoBoardDefinition(name: "head", connection: Discovered(First)),
                ]"#,
            )
            .unwrap(),
        };
        assert!(matches!(config.validate(), Err(ConfigError::DuplicateIoBoardName(name)) if name == "head"));

        let config = Config {
            cameras: vec![],
            io_boards: parse_io_boards(r#"[IoBoardDefinition(name: " ", connection: Discovered(First))]"#).unwrap(),
        };
        assert!(matches!(config.validate(), Err(ConfigError::EmptyIoBoardName)));
    }

    #[test]
    fn asset_configs_parse() {
        for content in [
            include_str!("../../assets/config/development-machine-1.ron"),
            include_str!("../../assets/config/development-machine-2.ron"),
        ] {
            ron::from_str::<Config>(content)
                .unwrap()
                .validate()
                .unwrap();
        }
    }
}
