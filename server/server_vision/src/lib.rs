use std::sync::Arc;
use chrono::DateTime;
use log::{debug, error, info};
use opencv::{imgcodecs, prelude::*};
use server_common::camera::{CameraDefinition, CameraSource};
use tokio::sync::broadcast;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

#[cfg(feature = "mediars-capture")]
pub mod mediars_capture;
#[cfg(feature = "opencv-capture")]
pub mod opencv_capture;

pub struct CameraFrame {
    pub frame_number: u64,
    pub jpeg_bytes: Vec<u8>,
    pub frame_timestamp: DateTime<chrono::Utc>,
}

pub fn dump_cameras() -> anyhow::Result<()> {
    #[cfg(feature = "mediars-capture")]
    let _ = mediars_capture::dump_cameras_mediars()
        .inspect_err(|e| error!("MediaRS camera error: {:?}", e.to_string()));

    #[cfg(feature = "opencv-capture")]
    let _ = opencv_capture::dump_cameras_opencv()
        .inspect_err(|e| error!("OpenCV cameras error: {:?}", e.to_string()));

    Ok::<(), anyhow::Error>(())
}


pub async fn capture_loop(
    tx: broadcast::Sender<Arc<CameraFrame>>,
    camera_definition: CameraDefinition,
    shutdown_flag: CancellationToken,
) -> anyhow::Result<()> {
    let capture_loop = make_capture_loop(&camera_definition, shutdown_flag)?;

    let callback = {
        let camera_definition = camera_definition.clone();

        move |frame: &'_ Mat, frame_timestamp, frame_instant, frame_duration: Duration, frame_number| {
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

            Ok::<(), ()>(())
        }
    };

    let result = match capture_loop {
        #[cfg(feature = "mediars-capture")]
        VideoCaptureImpl::MediaRS(mut loop_impl) => loop_impl.run(callback).await,
        #[cfg(feature = "opencv-capture")]
        VideoCaptureImpl::OpenCV(mut loop_impl) => loop_impl.run(callback).await,
        #[cfg(not(any(feature = "mediars-capture", feature = "opencv-capture")))]
        compile_error!("No camera capture implementation available") => { unreachable!() }
    };

    if let Err(e) = result {
        error!("Error in camera capture loop: {:?}", e);
    }

    info!("Shutting down camera capture. Camera: {:?}", camera_definition.source);

    Ok(())
}

fn make_capture_loop(
    camera_definition: &CameraDefinition,
    shutdown_flag: CancellationToken,
) -> anyhow::Result<VideoCaptureImpl> {
    match &camera_definition.source {
        #[cfg(feature = "opencv-capture")]
        CameraSource::OpenCV(_) => {
            let capture = opencv_capture::OpenCVCameraLoop::build(&camera_definition, shutdown_flag)?;

            Ok(VideoCaptureImpl::OpenCV(capture))
        }
        #[cfg(feature = "mediars-capture")]
        CameraSource::MediaRS(_) => {
            let capture = mediars_capture::MediaRSCameraLoop::build(&camera_definition, shutdown_flag)?;

            Ok(VideoCaptureImpl::MediaRS(capture))
        }
        _ => unimplemented!("Unsupported camera source: {:?}", camera_definition.source),
    }
}

/// Notes:
/// * not object-safe because it:
///   a) returns an `impl Future<...>` not a `Pin<Box<dyn Future<...>>>
///   b) has a generic parameter `F`.
/// * not being object-safe makes actually using this trait awkward when dealing with multiple implementations.
/// * adding the HRTB (Higher-rank trait bounds, `for<'a>` was required to get things compiling when using multiple implementations.
pub trait VideoCaptureLoop {
    // TODO make using this trait more ergonomic

    // TODO add a proper error type
    /// capture frames until canceled, calling the closure for each frame.
    ///
    /// caller can return an error, which may be logged, and allows the use of the `?` in the closure
    fn run<F>(&mut self, f: F) -> impl Future<Output = anyhow::Result<()>> + Send + '_
    where
        F: for<'a> Fn(&'a Mat, DateTime<chrono::Utc>, Instant, Duration, u64) -> Result<(), ()> + Send + Sync + 'static;
}

enum VideoCaptureImpl {
    #[cfg(feature = "mediars-capture")]
    MediaRS(mediars_capture::MediaRSCameraLoop),
    #[cfg(feature = "opencv-capture")]
    OpenCV(opencv_capture::OpenCVCameraLoop),
}
