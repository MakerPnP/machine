use std::collections::HashMap;
use std::pin::pin;
use std::time::Duration;

use eframe::epaint::ColorImage;
use egui::Context;
use ergot::toolkits::tokio_udp::EdgeStack;
use ergot::{Address, topic};
use image::ImageFormat;
use operator_shared::camera::{CameraCommand, CameraFrameChunk, CameraFrameChunkKind, CameraIdentifier};
use operator_shared::commands::OperatorCommandRequest;
use operator_shared::common::TimeStampUTC;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::watch::Sender;
use tokio::time::Instant;
use tracing::{debug, error, info, trace, warn};

use crate::events::AppEvent;
use crate::net::commands::OperatorCommandEndpoint;
use crate::{SCHEDULED_FPS_MAX, SCHEDULED_FPS_MIN, TARGET_FPS};

topic!(CameraFrameChunkTopic, CameraFrameChunk, "topic/camera_stream");

pub async fn camera_frame_listener(
    stack: EdgeStack,
    tx_out: Sender<CameraFrame>,
    context: Context,
    remote_address: Address,
    mut app_event_rx: broadcast::Receiver<AppEvent>,
    target_fps: u8,
) -> anyhow::Result<()> {
    let camera_identifier = CameraIdentifier::new(0);

    let command_client = stack
        .endpoints()
        .client::<OperatorCommandEndpoint>(remote_address, None);

    let subber = stack
        .topics()
        .bounded_receiver::<CameraFrameChunkTopic, 320>(None);
    let subber = pin!(subber);
    let mut hdl = subber.subscribe_unicast();
    let port_id = hdl.port();

    let result = command_client
        .request(&OperatorCommandRequest::CameraCommand(
            camera_identifier,
            CameraCommand::StartStreaming {
                port_id,
            },
        ))
        .await;
    if let Err(e) = result {
        return Err(anyhow::anyhow!("Error sending start request: {:?}", e));
    }

    struct InProgressFrame {
        total_chunks: u32,
        chunks: Vec<Option<Vec<u8>>>,
        received_count: u32,
        start_time: Instant,
        frame_timestamp: TimeStampUTC,
        frame_number: u64,
        frame_interval: Duration,
    }

    let mut in_progress: HashMap<u64, InProgressFrame> = HashMap::new();

    let mut target_fps = target_fps as f32;
    let mut frame_timestamps = std::collections::VecDeque::with_capacity(60);

    loop {
        select! {
            app_event = app_event_rx.recv() => {
                match app_event {
                    Ok(event) => match event {
                        AppEvent::Shutdown => {
                            break
                        }
                    }
                    Err(_) => {
                        break
                    }
                }
            }
            msg = hdl.recv() => {
                let chunk = &msg.t;

                let entry_and_image_chunk = match &chunk.kind {
                    CameraFrameChunkKind::Meta(frame_meta) => {


                        // Update timestamps for FPS estimation
                        frame_timestamps.push_back(frame_meta.frame_timestamp);
                        if frame_timestamps.len() > (TARGET_FPS * 2) as usize {
                            frame_timestamps.pop_front();
                        }

                        // Recompute effective FPS
                        if frame_timestamps.len() >= 2 {
                            let mut iter = frame_timestamps.iter();
                            let newest = iter.next_back().unwrap().0;
                            let previous: chrono::DateTime<chrono::Utc> = iter.next_back().unwrap().0;
                            let oldest = frame_timestamps.front().unwrap().0;

                            let newest_previous_span = newest - previous;
                            let total_span = newest - oldest;

                            let frame_count = frame_timestamps.len() - 1;
                            let measured_fps = frame_count as f64 / total_span.as_seconds_f64();


                            // Smooth update using exponential moving average
                            let alpha = 0.1;
                            target_fps = (1.0 - alpha) * target_fps + alpha * (measured_fps as f32);

                            debug!("estimated FPS: {:.1}, target FPS: {:.1}, total_span: {}ms, span: {}ms",
                                measured_fps,
                                target_fps,
                                total_span.num_milliseconds(),
                                newest_previous_span.num_milliseconds(),
                            );
                        }

                        // schedule next render
                        let clamped_fps = target_fps.clamp(SCHEDULED_FPS_MIN.into(), SCHEDULED_FPS_MAX.into());

                        let frame_interval = Duration::from_secs_f64(1.0 / clamped_fps as f64);

                        in_progress.insert(chunk.frame_number, InProgressFrame {
                            total_chunks: frame_meta.total_chunks,
                            chunks: vec![None; frame_meta.total_chunks as usize],
                            received_count: 0,
                            start_time: Instant::now(),
                            frame_timestamp: frame_meta.frame_timestamp.clone(),
                            frame_number: chunk.frame_number,
                            frame_interval,

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

                    debug!("received camera frame from server, frame_number: {}, chunks: {}, frame_timestamp: {:?}, frame_interval: {}ms", chunk.frame_number, entry.total_chunks, entry.frame_timestamp, entry.frame_interval.as_millis());

                    // Decode JPEG
                    let before = std::time::Instant::now();
                    match image::load_from_memory_with_format(&jpeg_data, ImageFormat::Jpeg) {
                        Ok(img) => {
                            let point1 = std::time::Instant::now();
                            let rgba = img.to_rgba8();
                            let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                            let color_image = ColorImage::from_rgba_unmultiplied([w, h], &rgba.into_raw());

                            let camera_frame = CameraFrame {
                                image: color_image,
                                timestamp: entry.frame_timestamp,
                                frame_number: entry.frame_number,
                                frame_interval: entry.frame_interval,
                            };

                            let _ = tx_out.send(camera_frame);
                            context.request_repaint();

                            let after = std::time::Instant::now();
                            trace!("sent frame to egui, frame_number: {}, size: {} bytes, decoding: {}us, imagegen+send: {}us, total-elapsed: {}us",
                                chunk.frame_number,
                                jpeg_data.len(),
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
    }

    let result = command_client
        .request(&OperatorCommandRequest::CameraCommand(
            camera_identifier,
            CameraCommand::StopStreaming {
                port_id,
            },
        ))
        .await;
    if let Err(e) = result {
        return Err(anyhow::anyhow!("Error sending stop request: {:?}", e));
    }
    info!("camera frame listener stopped, address: {}", remote_address);

    Ok(())
}

#[derive(Clone, Debug)]
pub struct CameraFrame {
    pub image: ColorImage,
    pub timestamp: TimeStampUTC,
    pub frame_number: u64,
    pub frame_interval: Duration,
}

impl Default for CameraFrame {
    fn default() -> Self {
        Self {
            image: Default::default(),
            timestamp: chrono::Utc::now().into(),
            frame_number: 0,
            frame_interval: Duration::from_secs(0),
        }
    }
}
