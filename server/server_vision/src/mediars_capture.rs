use std::time::Duration;
use chrono::{DateTime, Utc};
use opencv::core::Mat;
use opencv::videoio::VideoCapture;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use server_common::camera::{CameraDefinition, CameraSource};
use crate::VideoCaptureLoop;

pub fn mediars_camera(
    camera_definition: &CameraDefinition,
    shutdown_flag: CancellationToken,
) -> anyhow::Result<impl VideoCaptureLoop + use<>> {
    let CameraSource::OpenCV(open_cv_camera_config) = &camera_definition.source else {
        // not an OpenCV camera
        anyhow::bail!("Not an OpenCV camera")
    };

    Ok(MediaRSCameraLoop::new(camera_definition.fps, shutdown_flag))

}

struct MediaRSCameraLoop {
    fps: f32,
    shutdown_flag: CancellationToken,
}

impl MediaRSCameraLoop {
    pub fn new(fps: f32, shutdown_flag: CancellationToken) -> Self {
        Self {
            fps,
            shutdown_flag,
        }
    }
}

impl VideoCaptureLoop for MediaRSCameraLoop {
    fn run<F>(&mut self, f: F) -> impl Future<Output=anyhow::Result<()>> + Send + '_
    where
        F: Fn(&Mat, DateTime<Utc>, Instant, Duration, u64) -> Result<(), ()> + Send + Sync + 'static
    {
        async move {
            todo!()
        }
    }
}
