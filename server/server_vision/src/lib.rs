use std::sync::Arc;

use log::trace;
use opencv::{imgcodecs, prelude::*, videoio};
use server_common::camera::CameraDefinition;
use tokio::{
    sync::broadcast::Sender,
    time::{self, Duration},
};

pub struct CameraFrame {
    pub frame_number: u64,
    pub jpeg_bytes: Vec<u8>,
    pub frame_timestamp: chrono::DateTime<chrono::Utc>,
}

pub async fn capture_loop(tx: Sender<Arc<CameraFrame>>, camera_definition: CameraDefinition) -> anyhow::Result<()> {
    // Open default camera (index 0)
    let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?; // 0 = default device
    if !videoio::VideoCapture::is_opened(&cam)? {
        anyhow::bail!("Unable to open default camera");
    }
    cam.set(videoio::CAP_PROP_FRAME_WIDTH, f64::from(camera_definition.width))?;
    cam.set(videoio::CAP_PROP_FRAME_HEIGHT, f64::from(camera_definition.height))?;

    let period = Duration::from_millis((1000u16 / camera_definition.fps) as u64);
    let mut interval = time::interval(period);
    let mut frame_number = 0_u64;
    loop {
        interval.tick().await;
        let mut frame = Mat::default();
        cam.read(&mut frame)?;
        if frame.empty() {
            // skip or try again
            continue;
        }
        let frame_timestamp = chrono::Utc::now();

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
        // Ignore send error (no subscribers)
        let _ = tx.send(camera_frame_arc);

        let send_end = time::Instant::now();
        let send_duration = (send_end - send_start).as_micros() as u32;

        trace!(
            "now: {:?}, frame_number: {}, encode_duration: {}us, send_duration: {}us",
            time::Instant::now(),
            frame_number,
            encode_duration,
            send_duration
        );
        frame_number += 1;
    }
}
