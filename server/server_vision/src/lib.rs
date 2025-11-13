use std::sync::Arc;

use log::{debug, info, trace};
use opencv::videoio::VideoWriter;
use opencv::{imgcodecs, prelude::*, videoio};
use server_common::camera::CameraDefinition;
use tokio::sync::broadcast;
use tokio::time::{self, Duration};
use tokio_util::sync::CancellationToken;

pub struct CameraFrame {
    pub frame_number: u64,
    pub jpeg_bytes: Vec<u8>,
    pub frame_timestamp: chrono::DateTime<chrono::Utc>,
}

pub async fn capture_loop(
    tx: broadcast::Sender<Arc<CameraFrame>>,
    camera_definition: CameraDefinition,
    shutdown_flag: CancellationToken,
) -> anyhow::Result<()> {
    // Open default camera (index 0)
    let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?; // 0 = default device
    if !videoio::VideoCapture::is_opened(&cam)? {
        anyhow::bail!("Unable to open default camera");
    }
    info!(
        "Backend: {}",
        cam.get_backend_name()
            .unwrap_or("Unknown".to_string())
    );
    info!("GUID: {}", cam.get(videoio::CAP_PROP_GUID)?);
    info!("HW_DEVICE: {}", cam.get(videoio::CAP_PROP_HW_DEVICE)?);
    cam.set(videoio::CAP_PROP_FRAME_WIDTH, f64::from(camera_definition.width))?;
    cam.set(videoio::CAP_PROP_FRAME_HEIGHT, f64::from(camera_definition.height))?;
    cam.set(videoio::CAP_PROP_FPS, f64::from(camera_definition.fps))?;
    cam.set(videoio::CAP_PROP_BUFFERSIZE, f64::from(1))?;
    cam.set(videoio::CAP_PROP_FORMAT, f64::from(1))?;

    if let Some(four_cc) = camera_definition.four_cc {
        let four_cc_i32 = VideoWriter::fourcc(four_cc[0], four_cc[1], four_cc[2], four_cc[3])?;
        info!("FourCC: {:?} ({} / 0x{:08x})", four_cc, four_cc_i32, four_cc_i32);

        cam.set(videoio::CAP_PROP_FOURCC, f64::from(four_cc_i32))?;
    }

    let configured_fps = cam.get(videoio::CAP_PROP_FPS)? as f32;
    info!("Configured FPS: {}", configured_fps);

    let period = Duration::from_secs_f64(1.0 / configured_fps as f64);

    let mut interval = time::interval(period);
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    let mut previous_frame_at = time::Instant::now();
    let mut frame = Mat::default();

    let mut frame_number = 0_u64;
    loop {
        interval.tick().await;

        if tx.receiver_count() > 0 {
            let frame_timestamp = chrono::Utc::now();
            let frame_instant = time::Instant::now();

            cam.read(&mut frame)?;
            if frame.empty() {
                // skip or try again
                continue;
            }
            let frame_duration = (frame_instant - previous_frame_at).as_millis();
            previous_frame_at = frame_instant;

            // Encode to JPEG (quality default). You can set params to reduce quality/size.
            let encode_start = time::Instant::now();
            let mut buf = opencv::core::Vector::new();
            let params = opencv::core::Vector::new(); // default
            imgcodecs::imencode(".jpg", &frame, &mut buf, &params)?;

            let encode_end = time::Instant::now();
            let encode_duration = (encode_end - encode_start).as_micros() as u32;

            let send_start = time::Instant::now();

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

            let send_end = time::Instant::now();
            let send_duration = (send_end - send_start).as_micros() as u32;

            debug!(
                "frame_timestamp: {:?}, frame_number: {}, encode_duration: {}us, send_duration: {}us, frame_duration: {}ms",
                frame_timestamp, frame_number, encode_duration, send_duration, frame_duration
            );
            frame_number += 1;
        }

        if shutdown_flag.is_cancelled() {
            info!("Shutting down camera capture");
            break;
        }
    }

    Ok(())
}
