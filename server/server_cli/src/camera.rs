use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use ergot::interface_manager::InterfaceSendError;
use ergot::interface_manager::interface_impls::tokio_udp::TokioUdpInterface;
use ergot::interface_manager::profiles::direct_router::DirectRouter;
use ergot::net_stack::ArcNetStack;
use ergot::{Address, NetStackSendError, topic};
use log::{debug, error, info, trace};
use mutex::raw_impls::cs::CriticalSectionRawMutex;
use operator_shared::camera::{
    CameraFrameChunk, CameraFrameChunkKind, CameraFrameImageChunk, CameraFrameMeta, CameraIdentifier,
};
use server_common::camera::CameraDefinition;
use server_vision::CameraFrame;
use tokio::sync::broadcast;
use tokio::{select, time};
use tokio_util::sync::CancellationToken;

topic!(CameraFrameChunkTopic, CameraFrameChunk, "topic/camera_stream");

pub async fn camera_streamer(
    stack: ArcNetStack<CriticalSectionRawMutex, DirectRouter<TokioUdpInterface>>,
    mut rx: broadcast::Receiver<Arc<CameraFrame>>,
    definition: CameraDefinition,
    chunk_size: usize,
    address: Address,
    shutdown_flag: CancellationToken,
    // the target fps of the camera stream.  which may be lower than the actual fps of the camera
    target_fps: u8,
) -> Result<()> {
    info!("camera streamer started. destination: {}", address);

    let mut interval = time::interval(Duration::from_secs(1));
    let mut next_frame_at = time::Instant::now();
    let target_fps_interval = Duration::from_secs_f32(1.0 / target_fps as f32);

    loop {
        select! {
            _ = interval.tick() => {
                if shutdown_flag.is_cancelled() {
                    info!("Shutting down camera streamer");
                    break
                }
            }
            frame = rx.recv() => {
                let now = time::Instant::now();
                if now < next_frame_at {
                    // skip this frame, the client requested a lower frame rate.
                    continue;
                }

                // Receive oldest frame (await)
                let camera_frame = match frame {
                    Ok(b) => b,
                    Err(broadcast::error::RecvError::Lagged(skipped_frames)) => {
                        // If lagged, try to get the next available
                        debug!("lagged, trying to get next frame.  skipped: {}", skipped_frames);
                        continue;
                    }
                    Err(e) => return Err(anyhow::anyhow!(e)),
                };

                let CameraFrame { frame_number, jpeg_bytes, frame_timestamp } = &*camera_frame;

                let total_bytes = jpeg_bytes.len() as u32;
                let total_chunks = (total_bytes + (chunk_size as u32) - 1) / chunk_size as u32;

                trace!("Sending frame, now: {:?}, frame_number: {}, total_chunks: {}, len: {}", now, camera_frame.frame_number, total_chunks, total_bytes);

                let frame_chunk = CameraFrameChunk {
                    frame_number: *frame_number,
                    kind: CameraFrameChunkKind::Meta(CameraFrameMeta {
                        total_chunks,
                        total_bytes,
                        frame_timestamp: (*frame_timestamp).into(),
                    })
                };
                if stack.topics().unicast_borrowed::<CameraFrameChunkTopic>(address, &frame_chunk).is_err() {
                    trace!("Unable to send first frame chunk. frame_number: {}", frame_number);
                    // no point even trying to send the chunks if the first chunk failed, drop the frame
                    continue
                }

                let mut ok = true;
                for (chunk_index, chunk) in jpeg_bytes.chunks(chunk_size).enumerate() {
                    let frame_chunk = CameraFrameChunk {
                        frame_number: *frame_number,
                        kind: CameraFrameChunkKind::ImageChunk(CameraFrameImageChunk {
                            chunk_index: chunk_index as u32,
                            bytes: chunk.to_vec(),
                        })
                    };

                    let chunk_start_at = time::Instant::now();

                    // IMPORTANT: back-off delay needs to be as short as possible
                    //            60fps =  16ms total frame time.
                    //            30fps =  33ms total frame time.
                    //            25fps =  40ms total frame time.
                    //            15fps =  66ms total frame time.
                    //            10fps = 100ms total frame time.
                    const INITIAL_BACKOFF: Duration = Duration::from_micros(100);
                    let mut retries = 0;

                    let result = loop {
                        match stack.topics().unicast_borrowed::<CameraFrameChunkTopic>(address, &frame_chunk) {
                            r @ Ok(_) => {
                                // reset
                                break r
                            }
                            e1 @ Err(NetStackSendError::InterfaceSend(InterfaceSendError::InterfaceFull)) => {
                                if chunk_start_at.elapsed() > Duration::from_millis(100) {
                                    break e1
                                } else {
                                    let backoff = INITIAL_BACKOFF * (1 << retries.min(4));
                                    time::sleep_until(chunk_start_at + backoff).await;
                                }
                            }
                            e2@ Err(_) => {
                                break e2
                            }
                        }

                        retries += 1;
                    };

                    match result {
                        Ok(_) => tokio::task::yield_now().await,
                        Err(e) => {
                            error!("Aborting frame, error sending chunk. frame_number: {}, chunk: {}/{}, retries: {}, error: {:?}", frame_number, chunk_index + 1, total_chunks, retries, e);
                            ok = false;
                            break
                        }
                    }
                }

                if ok {
                    trace!("Frame sent. frame_number: {}", frame_number);

                    // if sending the frame failed, we need to send the next-received frame immediately
                    // we only update the `next_frame_at` if the frame was successfully sent.

                    let now = time::Instant::now();
                    next_frame_at += target_fps_interval;
                    if now > next_frame_at {
                        // catch up if we fall behind
                        next_frame_at = now + target_fps_interval;
                    }

                }

            }
        }
    }

    Ok(())
}

pub fn camera_definition_for_identifier<'a>(
    definitions: &'a Vec<CameraDefinition>,
    identifier: &CameraIdentifier,
) -> Option<&'a CameraDefinition> {
    // for now, just using the identifier as an index
    let index: u8 = **identifier;
    definitions.get(index as usize)
}
