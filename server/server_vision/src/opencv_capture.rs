use std::time::Duration;

use chrono::DateTime;
use log::{error, info};
use opencv::core::Mat;
use opencv::videoio::{VideoCapture, VideoWriter};
use opencv::{prelude::*, videoio};
use server_common::camera::{CameraDefinition, CameraSource};
use tokio::time;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::VideoCaptureLoop;

pub struct OpenCVCameraLoop {
    fps: f32,
    cam: VideoCapture,
    shutdown_flag: CancellationToken,
}

impl OpenCVCameraLoop {
    pub fn build(
        camera_definition: &CameraDefinition,
        shutdown_flag: CancellationToken,
    ) -> anyhow::Result<Self> {
        let CameraSource::OpenCV(open_cv_camera_config) = &camera_definition.source else {
            anyhow::bail!("Not an OpenCV camera")
        };

        // Open default camera (index 0)
        let mut cam: VideoCapture = VideoCapture::new(open_cv_camera_config.index, videoio::CAP_ANY)?; // 0 = default device
        if !VideoCapture::is_opened(&cam)? {
            anyhow::bail!(
                "Unable to open OpenCV camera. OpenCVCamera: {}",
                open_cv_camera_config.index
            );
        }
        info!(
            "OpenCVCamera: {}, GUID: {}, HW_DEVICE: {}, Backend: {}",
            open_cv_camera_config.index,
            cam.get(videoio::CAP_PROP_GUID)?,
            cam.get(videoio::CAP_PROP_HW_DEVICE)?,
            cam.get_backend_name()
                .unwrap_or("Unknown".to_string())
        );
        cam.set(videoio::CAP_PROP_FRAME_WIDTH, f64::from(camera_definition.width))?;
        cam.set(videoio::CAP_PROP_FRAME_HEIGHT, f64::from(camera_definition.height))?;
        cam.set(videoio::CAP_PROP_FPS, f64::from(camera_definition.fps))?;
        cam.set(videoio::CAP_PROP_BUFFERSIZE, f64::from(1))?;
        cam.set(videoio::CAP_PROP_FORMAT, f64::from(1))?;

        if let Some(four_cc) = camera_definition.four_cc {
            let four_cc_i32 = VideoWriter::fourcc(four_cc[0], four_cc[1], four_cc[2], four_cc[3])?;
                info!(
                "OpenCVCamera: {}, FourCC: {:?} ({} / 0x{:08x})",
                open_cv_camera_config.index, four_cc, four_cc_i32, four_cc_i32
            );
        
            cam.set(videoio::CAP_PROP_FOURCC, f64::from(four_cc_i32))?;
        }
    
        let configured_fps = cam.get(videoio::CAP_PROP_FPS)? as f32;
        info!(
            "OpenCVCamera: {}, Configured FPS: {}",
            open_cv_camera_config.index, configured_fps
        );
        
        Ok(Self {
            fps: configured_fps,
            cam,
            shutdown_flag,
        })
    }
}

impl VideoCaptureLoop for OpenCVCameraLoop {
    fn run<F>(&mut self, f: F) -> impl Future<Output = anyhow::Result<()>> + Send + '_
    where
        F: for<'a> Fn(&'a Mat, DateTime<chrono::Utc>, Instant, Duration, u64) -> Result<(), ()> + Send + Sync + 'static,
    {
        async move {
            let mut frame_number = 0_u64;

            let period = Duration::from_secs_f64(1.0 / self.fps as f64);

            let mut interval = time::interval(period);
            interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

            let mut previous_frame_at = Instant::now();
            let mut frame_mat = Mat::default();

            loop {
                interval.tick().await;

                let frame_timestamp = chrono::Utc::now();
                let frame_instant = Instant::now();

                self.cam.read(&mut frame_mat)?;
                if frame_mat.empty() {
                    // skip or try again
                    continue;
                }

                frame_number += 1;

                let frame_duration = frame_instant - previous_frame_at;
                previous_frame_at = frame_instant;

                let result = f(&frame_mat, frame_timestamp, frame_instant, frame_duration, frame_number);
                if result.is_err() {
                    error!("OpenCV frame processing error: {:?}", result);
                }

                if self.shutdown_flag.is_cancelled() {
                    break;
                }
            }

            Ok(())
        }
    }
}

#[cfg(feature = "opencv-capture")]
pub fn dump_cameras_opencv() -> anyhow::Result<()>{
    anyhow::bail!("Unsupported for OpenCV");
}
