use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};
use log::{debug, error, info};
use media::device::backend::media_foundation::{MediaFoundationDevice, MediaFoundationDeviceManager};
use media::device::camera::{CameraManager, DefaultCameraManager};
use media::device::{Device, DeviceManager, OutputDevice};
use media::FrameDescriptor;
use media::video::PixelFormat;
#[cfg(feature = "opencv-411")]
use opencv::core::AlgorithmHint;
use opencv::core::{CV_8UC1, CV_8UC2, CV_8UC3, CV_8UC4, Vector};
use opencv::imgproc;
use opencv::imgproc::{
    COLOR_YUV2BGR_I420, COLOR_YUV2BGR_NV12, COLOR_YUV2BGR_UYVY, COLOR_YUV2BGR_YUY2,
    COLOR_YUV2BGR_YVYU,
};
use opencv::prelude::*;
use server_common::camera::{CameraDefinition, CameraSource};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::VideoCaptureLoop;

pub struct MediaRSCameraLoop {
    fps: f32,
    shutdown_flag: CancellationToken,
    device: Arc<Mutex<&'static mut <DefaultCameraManager as DeviceManager>::DeviceType>>,
    cam_mgr: CameraManager<DefaultCameraManager>,
}

// Safety: the cam_mgr and device are only used by a single thread, right?
unsafe impl Send for MediaRSCameraLoop {}

impl MediaRSCameraLoop {
    pub fn build(
        camera_definition: &CameraDefinition,
        shutdown_flag: CancellationToken,
    ) -> anyhow::Result<Self> {
        let CameraSource::MediaRS(media_rs_camera_config) = &camera_definition.source else {
            anyhow::bail!("Not a MediaRS camera")
        };

        let mut cam_mgr = match CameraManager::new_default() {
            Ok(cam_mgr) => cam_mgr,
            Err(e) => {
                anyhow::bail!("{:?}", e.to_string());
            }
        };

        // Get the first camera
        let device = match cam_mgr.lookup_mut(&media_rs_camera_config.device_id) {
            Some(device) => device,
            None => {
                anyhow::bail!("No camera found with id: {}", media_rs_camera_config.device_id);
            }
        };
        // transmute so we can store the device and the camera camera manager we borrowed it from in Self
        let device: &'static mut <DefaultCameraManager as DeviceManager>::DeviceType = unsafe { std::mem::transmute(device) };

        Ok(Self {
            fps: 30.0,
            shutdown_flag,
            cam_mgr,
            device: Arc::new(Mutex::new(device)),
        })
    }
}

impl VideoCaptureLoop for MediaRSCameraLoop {
    fn run<F>(&mut self, f: F) -> impl Future<Output = anyhow::Result<()>> + Send + '_
    where
        F: for<'b> Fn(&'b Mat, DateTime<Utc>, Instant, Duration, u64) -> Result<(), ()> + Send + Sync + 'static,
    {

        async move {

            if let Err(e) = self.device.lock().unwrap().set_output_handler({
                let fps = self.fps;
                move |frame| {
                    debug!("frame source: {:?}", frame.source);
                    debug!("frame desc: {:?}", frame.descriptor());
                    debug!("frame duration: {:?}", frame.duration);

                    let capture_timestamp = chrono::Utc::now();
                    let capture_instant = Instant::now();

                    // TODO get this from the frame
                    let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
                    // TODO handle this somehow
                    let frame_number = 0;

                    // Map the video frame for memory access
                    if let Ok(mapped_guard) = frame.map() {
                        if let Some(planes) = mapped_guard.planes() {
                            for (index, plane) in planes.into_iter().enumerate() {
                                debug!(
                                    "plane. index: {}, stride: {:?}, height: {:?}",
                                    index,
                                    plane.stride(),
                                    plane.height()
                                );
                            }

                            process_frame(&frame, |mat| {
                                let _ = f(&mat, capture_timestamp, capture_instant, frame_duration, frame_number);
                            });
                        }
                    }
                    Ok(())
                }
            }) {
                error!("{:?}", e.to_string());
            }

            {
                let mut device = self.device.lock().unwrap();
                // Start the camera
                if let Err(e) = device.start() {
                    error!("{:?}", e.to_string());
                }
            }

            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;

                if self.shutdown_flag.is_cancelled() {
                    break;
                }
            }

            {
                let mut device = self.device.lock().unwrap();


                info!("Stopping camera: {}", device.id());
                // Stop the camera
                if let Err(e) = device.stop() {
                    error!("{:?}", e.to_string());
                }
            }

            Ok(())
        }
    }
}


fn process_frame<'a, F>(frame: &'a media::frame::Frame, mut f: F)
where
    F: for<'b> FnMut(Mat),
{
    let mapped = frame.map().unwrap();
    let planes = mapped.planes().unwrap();

    let FrameDescriptor::Video(vfd) = frame.descriptor() else {
        panic!("unsupported frame type");
    };

    // Get format information and create appropriate OpenCV Mat
    let cv_type = match vfd.format {
        PixelFormat::YUYV => Some(CV_8UC2), // YUY2, YUYV: Y0 Cb Y1 Cr (YUV 4:2:2)
        PixelFormat::UYVY => Some(CV_8UC2), // UYVY: Cb Y0 Cr Y1 (YUV 4:2:2)
        PixelFormat::YVYU => Some(CV_8UC2), // YVYU: Y0 Cr Y1 Cb (YUV 4:2:2)
        PixelFormat::VYUY => Some(CV_8UC2), // VYUY: Cr Y0 Cb Y1 (YUV 4:2:2)
        PixelFormat::RGB24 => Some(CV_8UC3), // RGB 24-bit (8-bit per channel)
        PixelFormat::BGR24 => Some(CV_8UC3), // BGR 24-bit (8-bit per channel)
        PixelFormat::ARGB32 => Some(CV_8UC4), // ARGB 32-bit
        PixelFormat::BGRA32 => Some(CV_8UC4), // BGRA 32-bit
        PixelFormat::RGBA32 => Some(CV_8UC4), // RGBA 32-bit
        PixelFormat::ABGR32 => Some(CV_8UC4), // ABGR 32-bit
        PixelFormat::Y8 => Some(CV_8UC1),   // Grayscale 8-bit
        _ => None,
    };

    let width = vfd.width.get();
    let height = vfd.height.get();

    // Handle different pixel formats appropriately
    let bgr_mat = match (vfd.format, cv_type) {
        (
            PixelFormat::YUYV | PixelFormat::UYVY | PixelFormat::YVYU | PixelFormat::VYUY,
            Some(cv_type),
        ) => {
            let plane = planes.into_iter().next().unwrap();
            let data = plane.data().unwrap();
            let stride = plane.stride().unwrap();

            let code = match vfd.format {
                PixelFormat::YUYV => COLOR_YUV2BGR_YUY2,
                PixelFormat::YVYU => COLOR_YUV2BGR_YVYU,
                PixelFormat::UYVY => COLOR_YUV2BGR_UYVY,
                PixelFormat::VYUY => COLOR_YUV2BGR_YUY2,
                _ => unreachable!(),
            };

            let raw_mat = unsafe {
                Mat::new_rows_cols_with_data_unsafe(
                    height as i32,
                    width as i32,
                    cv_type, // 2 channels per pixel
                    data.as_ptr() as *mut std::ffi::c_void,
                    stride as usize, // step (bytes per row)
                )
                    .unwrap()
            };

            // Convert UYVY to BGR
            let mut bgr_mat =
                unsafe { Mat::new_rows_cols(height as i32, width as i32, CV_8UC3) }.unwrap();
            #[cfg(feature = "opencv-410")]
            imgproc::cvt_color(&raw_mat, &mut bgr_mat, code, 0).unwrap();
            #[cfg(feature = "opencv-411")]
            imgproc::cvt_color(
                &raw_mat,
                &mut bgr_mat,
                code,
                0,
                AlgorithmHint::ALGO_HINT_DEFAULT,
            )
                .unwrap();

            bgr_mat
        }
        (PixelFormat::RGB24 | PixelFormat::BGR24, Some(cv_type)) => {
            let plane = planes.into_iter().next().unwrap();
            let data = plane.data().unwrap();
            let stride = plane.stride().unwrap();

            let raw_mat = unsafe {
                Mat::new_rows_cols_with_data_unsafe(
                    height as i32,
                    width as i32,
                    cv_type,
                    data.as_ptr() as *mut std::ffi::c_void,
                    stride as usize,
                )
                    .unwrap()
            };

            // For RGB24, convert to BGR if needed for OpenCV processing
            if vfd.format == PixelFormat::RGB24 {
                let mut bgr_mat =
                    unsafe { Mat::new_rows_cols(height as i32, width as i32, CV_8UC3) }.unwrap();
                #[cfg(feature = "opencv-410")]
                imgproc::cvt_color(&raw_mat, &mut bgr_mat, imgproc::COLOR_RGB2BGR, 0).unwrap();
                #[cfg(feature = "opencv-411")]
                imgproc::cvt_color(
                    &raw_mat,
                    &mut bgr_mat,
                    imgproc::COLOR_RGB2BGR,
                    0,
                    AlgorithmHint::ALGO_HINT_DEFAULT,
                )
                    .unwrap();
                bgr_mat
            } else {
                raw_mat.try_clone().unwrap()
            }
        }
        (PixelFormat::NV12, None) => {
            // Get Y plane (first plane) and UV plane (second plane)
            let mut planes_iter = planes.into_iter();
            let y_plane = planes_iter.next().unwrap();
            let uv_plane = planes_iter.next().unwrap();

            let y_data = y_plane.data().unwrap();
            let uv_data = uv_plane.data().unwrap();

            let y_stride = y_plane.stride().unwrap();
            let uv_stride = uv_plane.stride().unwrap();

            // Create mats for both planes
            let y_mat = unsafe {
                Mat::new_rows_cols_with_data_unsafe(
                    height as i32,
                    width as i32,
                    CV_8UC1,
                    y_data.as_ptr() as *mut std::ffi::c_void,
                    y_stride as usize,
                )
                    .unwrap()
            };

            // UV plane has half the height and potentially a different stride
            let uv_mat = unsafe {
                Mat::new_rows_cols_with_data_unsafe(
                    (height / 2) as i32,
                    (width / 2) as i32,
                    CV_8UC2, // Interleaved U and V
                    uv_data.as_ptr() as *mut std::ffi::c_void,
                    uv_stride as usize,
                )
                    .unwrap()
            };

            // Create a BGR mat for output
            let mut bgr_mat =
                unsafe { Mat::new_rows_cols(height as i32, width as i32, CV_8UC3) }.unwrap();

            // Method 1: Use OpenCV's cvtColorTwoPlane
            // This function explicitly converts from separate Y and UV planes
            #[cfg(feature = "opencv-410")]
            imgproc::cvt_color_two_plane(&y_mat, &uv_mat, &mut bgr_mat, COLOR_YUV2BGR_NV12)
                .unwrap();
            #[cfg(feature = "opencv-411")]
            imgproc::cvt_color_two_plane(
                &y_mat,
                &uv_mat,
                &mut bgr_mat,
                COLOR_YUV2BGR_NV12,
                AlgorithmHint::ALGO_HINT_DEFAULT,
            )
                .unwrap();

            bgr_mat
        }
        // I420 (YUV 4:2:0 planar)
        (PixelFormat::I420, None) => {
            // Get the three planes: Y, U, V
            let mut planes_iter = planes.into_iter();
            let y_plane = planes_iter.next().unwrap();
            let u_plane = planes_iter.next().unwrap();
            let v_plane = planes_iter.next().unwrap();

            let y_data = y_plane.data().unwrap();
            let u_data = u_plane.data().unwrap();
            let v_data = v_plane.data().unwrap();

            let y_stride = y_plane.stride().unwrap();
            let u_stride = u_plane.stride().unwrap();
            let v_stride = v_plane.stride().unwrap();

            let height = height as usize;
            let width = width as usize;
            let uv_h = height / 2;
            let uv_w = width / 2;

            info!(
                "y_stride: {}, v_stride: {}, u_stride: {}",
                y_stride, v_stride, u_stride
            );
            info!(
                "width: {}, height: {}, uv_w: {}, uv_h: {}",
                width, height, uv_w, uv_h
            );

            // Calculate total size needed for I420 contiguous buffer
            let y_size = y_stride * height;
            let u_size = u_stride * uv_h;
            let v_size = v_stride * uv_h;
            let total_size = y_size + u_size + v_size;

            // FUTURE avoid allocating and de-allocating the buffer for every frame, re-use it.

            // Create a contiguous buffer (we need to copy for correct layout)
            let mut i420_data = Vec::with_capacity(total_size);

            // Copy Y plane
            for row in 0..height {
                let start = row * y_stride;
                let end = start + width;
                i420_data.extend_from_slice(&y_data[start..end]);
            }

            // Copy U plane
            for row in 0..uv_h {
                let start = row * u_stride;
                let end = start + uv_w;
                i420_data.extend_from_slice(&u_data[start..end]);
            }

            // Copy V plane
            for row in 0..uv_h {
                let start = row * v_stride;
                let end = start + uv_w;
                i420_data.extend_from_slice(&v_data[start..end]);
            }

            // Create Mat from contiguous I420 data
            let i420_mat = unsafe {
                Mat::new_rows_cols_with_data_unsafe(
                    (height * 3 / 2) as i32, // I420 has 1.5x height for planar data
                    width as i32,
                    CV_8UC1,
                    i420_data.as_ptr() as *mut c_void,
                    width, // stride is width for contiguous buffer
                )
                    .unwrap()
            };

            // Convert to BGR
            let mut bgr_mat = Mat::default();

            // Convert to BGR
            #[cfg(feature = "opencv-410")]
            imgproc::cvt_color(&i420_mat, &mut bgr_mat, COLOR_YUV2BGR_I420, 0).unwrap();
            #[cfg(feature = "opencv-411")]
            imgproc::cvt_color(
                &i420_mat,
                &mut bgr_mat,
                COLOR_YUV2BGR_I420,
                0,
                AlgorithmHint::ALGO_HINT_DEFAULT,
            )
                .unwrap();

            // Keep the vector alive until we're done with the Mat
            std::mem::forget(i420_data);

            bgr_mat
        }
        _ => {
            panic!(
                "Unsupported pixel format: {:?}. Common formats include YUYV, UYVY, NV12, I420, RGB24, BGR24",
                vfd.format
            );
        }
    };

    f(bgr_mat);
}
