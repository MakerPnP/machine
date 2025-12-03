use std::time::Duration;

use chrono::{DateTime, Utc};
use opencv::core::Mat;
use server_common::camera::{CameraDefinition, CameraSource};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::VideoCaptureLoop;

pub fn mediars_camera(
    camera_definition: &CameraDefinition,
    shutdown_flag: CancellationToken,
) -> anyhow::Result<MediaRSCameraLoop> {
    let CameraSource::MediaRS(media_rs_camera_config) = &camera_definition.source else {
        anyhow::bail!("Not a MediaRS camera")
    };

    Ok(MediaRSCameraLoop::new(camera_definition.fps, shutdown_flag))
}

pub struct MediaRSCameraLoop {
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
    fn run<F>(&mut self, f: F) -> impl Future<Output = anyhow::Result<()>> + Send + '_
    where
        F: for<'a> Fn(&'a Mat, DateTime<Utc>, Instant, Duration, u64) -> Result<(), ()> + Send + Sync + 'static,
    {
        async move { todo!() }
    }
}
