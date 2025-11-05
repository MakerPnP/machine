use std::collections::HashMap;
use std::pin::pin;
use std::time::Duration;
use eframe::epaint::ColorImage;
use egui::Context;
use ergot::toolkits::tokio_udp::EdgeStack;
use ergot::topic;
use image::ImageFormat;
use tracing::{debug, error, trace, warn};
use operator_shared::camera::{CameraFrameChunk, CameraFrameChunkKind};
use tokio::sync::watch::Sender;
use tokio::time::Instant;

topic!(CameraFrameChunkTopic, CameraFrameChunk, "topic/camera_stream");

pub async fn camera_frame_listener(stack: EdgeStack, id: u8, tx_out: Sender<ColorImage>, context: Context) {
    let subber = stack.topics().bounded_receiver::<CameraFrameChunkTopic, 320>(None);
    let subber = pin!(subber);
    let mut hdl = subber.subscribe();

    struct InProgressFrame {
        total_chunks: u32,
        chunks: Vec<Option<Vec<u8>>>,
        received_count: u32,
        start_time: Instant,
    }

    let mut in_progress: HashMap<u64, InProgressFrame> = HashMap::new();

    loop {
        let msg = hdl.recv().await;

        let chunk = &msg.t;

        let entry_and_image_chunk = match &chunk.kind {
            CameraFrameChunkKind::Meta(frame_meta) => {
                in_progress.insert(chunk.frame_number, InProgressFrame {
                    total_chunks: frame_meta.total_chunks,
                    chunks: vec![None; frame_meta.total_chunks as usize],
                    received_count: 0,
                    start_time: Instant::now(),
                });
                continue;
            }
            CameraFrameChunkKind::ImageChunk(image_chunk) => {
                in_progress.get_mut(&chunk.frame_number).map(|entry|(entry, image_chunk))
            }
        };

        let Some((entry, image_chunk)) = entry_and_image_chunk else {
            continue;
        };

        trace!(
            "received frame chunk: frame={} chunk={}/{} size={}",
            chunk.frame_number,
            image_chunk.chunk_index + 1,
            entry.total_chunks,
            image_chunk.bytes.len()
        );

        // Insert chunk if not already present
        let idx = image_chunk.chunk_index as usize;
        if idx >= entry.chunks.len() {
            trace!("invalid chunk index {} for frame {}", idx, chunk.frame_number);
            continue;
        }
        if entry.chunks[idx].is_none() {
            entry.chunks[idx] = Some(image_chunk.bytes.clone());
            entry.received_count += 1;
        }

        // Check if frame is complete
        if entry.received_count == entry.total_chunks {
            // Reassemble JPEG data in order
            let mut jpeg_data = Vec::new();
            for c in entry.chunks.iter() {
                if let Some(bytes) = c {
                    jpeg_data.extend_from_slice(bytes);
                } else {
                    // Missing chunk — shouldn’t happen
                    trace!("missing chunk during reassembly for frame {}", chunk.frame_number);
                    continue;
                }
            }

            let before = std::time::Instant::now();
            debug!("received camera frame from server, frame_number: {}, chunks: {}, timestamp: {:?}", chunk.frame_number, entry.total_chunks, before);

            // Decode JPEG
            let before = std::time::Instant::now();
            match image::load_from_memory_with_format(&jpeg_data, ImageFormat::Jpeg) {
                Ok(img) => {
                    let point1 = std::time::Instant::now();
                    let rgba = img.to_rgba8();
                    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                    let color_image = ColorImage::from_rgba_unmultiplied([w, h], &rgba.into_raw());

                    let _ = tx_out.send(color_image);
                    context.request_repaint();

                    let after = std::time::Instant::now();
                    trace!("sent frame to egui, frame_number: {}, size: {} bytes, timestamp: {:?}, decoding: {}us, imagegen+send: {}us, total-elapsed: {}us",
                        chunk.frame_number,
                        jpeg_data.len(),
                        after,
                        (point1 - before).as_micros(),
                        (after - point1).as_micros(),
                        (after - before).as_micros(),
                    );
                }
                Err(e) => {
                    error!("decode error frame {}: {:?}", chunk.frame_number, e);
                }
            }


            // Remove the completed frame from tracking
            in_progress.remove(&chunk.frame_number);
        }
        // drop old frames (stuck/incomplete)
        let now = Instant::now();
        in_progress.retain(|frame_num, f| {
            if now.duration_since(f.start_time) > Duration::from_secs(1) {
                warn!(
                        "discarding incomplete frame {} (got {}/{})",
                        frame_num,
                        f.received_count,
                        f.total_chunks
                    );
                false
            } else {
                true
            }
        });
    }
}
