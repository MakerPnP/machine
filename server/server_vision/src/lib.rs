use std::sync::Arc;

use chrono::DateTime;
use log::{debug, error, info};
use opencv::videoio::{VideoCapture, VideoWriter};
use opencv::{imgcodecs, prelude::*, videoio};
use server_common::camera::{CameraDefinition, CameraSource};
use tokio::sync::broadcast;
use tokio::time::{self, Duration, Instant};
use tokio_util::sync::CancellationToken;

#[cfg(feature ="opencv-capture")]
pub mod opencv_capture;

pub struct CameraFrame {
    pub frame_number: u64,
    pub jpeg_bytes: Vec<u8>,
    pub frame_timestamp: DateTime<chrono::Utc>,
}

pub async fn capture_loop(
    tx: broadcast::Sender<Arc<CameraFrame>>,
    camera_definition: CameraDefinition,
    shutdown_flag: CancellationToken,
) -> anyhow::Result<()> {
    #[cfg(feature ="opencv-capture")]
    let mut capture_loop = opencv_capture::opencv_camera(&camera_definition, shutdown_flag)?;
    #[cfg(not(feature ="opencv-capture"))]
    unimplemented!("OpenCV capture not enabled, currently only OpenCV is supported.");

    let result = capture_loop.run({
        let camera_definition = camera_definition.clone();

        move |frame, frame_timestamp, frame_instant, frame_duration, frame_number| {
            if tx.receiver_count() > 0 {
                // Encode to JPEG (quality default). You can set params to reduce quality/size.
                let encode_start = Instant::now();
                let mut buf = opencv::core::Vector::new();
                let params = opencv::core::Vector::new(); // default
                imgcodecs::imencode(".jpg", &frame, &mut buf, &params)
                    .map_err(|e| error!("OpenCV imencode error: {:?}", e))?;

                let encode_end = Instant::now();
                let encode_duration = (encode_end - encode_start).as_micros() as u32;

                let send_start = Instant::now();

                // Wrap bytes into Arc so broadcast clones cheap
                let camera_frame = CameraFrame {
                    frame_number,
                    jpeg_bytes: buf.to_vec(),
                    frame_timestamp,
                };

                let camera_frame_arc = Arc::new(camera_frame);
                // safe to ignore the error, no subscribers yet, however we're only sending a frame if we
                // have subscribers, so this should never fail anyway.
                let _ = tx.send(camera_frame_arc);

                let send_end = Instant::now();
                let send_duration = (send_end - send_start).as_micros() as u32;

                debug!(
                    "Camera: {:?}, frame_timestamp: {:?}, frame_number: {}, encode_duration: {}us, send_duration: {}us, frame_duration: {}us",
                    camera_definition.source,
                    frame_timestamp,
                    frame_number,
                    encode_duration,
                    send_duration,
                    frame_duration.as_micros()
                );
            }

            Ok(())
        }
    }).await;

    if let Err(e) = result {
        error!("Error in camera capture loop: {:?}", e);
    }

    info!("Shutting down camera capture. Camera: {:?}", camera_definition.source);

    Ok(())
}

pub trait VideoCaptureLoop {
    // TODO add a proper error type
    /// capture frames until canceled, calling the closure for each frame.
    ///
    /// caller can return an error, which may be logged, and allows the use of the `?` in the closure
    fn run<F>(&mut self, f: F) -> impl Future<Output = anyhow::Result<()>> + Send + '_
    where
        F: Fn(&Mat, DateTime<chrono::Utc>, Instant, Duration, u64) -> Result<(), ()> + Send + Sync + 'static;
}
