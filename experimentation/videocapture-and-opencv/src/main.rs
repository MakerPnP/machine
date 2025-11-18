//! Demonstrates how to use VideoCapture + OpenCV with egui.
//!
//! Does NOT use the OpenCV feature `videoio` to capture video frames.
//!
//! This example uses the [video_capture](https://crates.io/crates/video_capture) crate to capture video frames from a camera.
//! The frames are then processed using OpenCV and the results are displayed using egui.
//!
//! The OpenCV face detection classifier is used to detect faces in the video frames.
//!
//! References:
//! https://learn.microsoft.com/en-us/windows/win32/medfound/recommended-8-bit-yuv-formats-for-video-rendering
//! https://www.itu.int/dms_pubrec/itu-r/rec/bt/r-rec-bt.601-7-201103-i!!pdf-e.pdf

use eframe::epaint::StrokeKind;
use eframe::{CreationContext, Frame};
use egui::{ColorImage, Context, Pos2, RichText, TextureHandle, UiBuilder, Vec2, Widget};
use log::{debug, error, info};
use opencv::core::{AlgorithmHint, CV_8UC1, CV_8UC2, CV_8UC3, CV_8UC4, Vector};
use opencv::imgproc;
use opencv::imgproc::{
    COLOR_YUV2BGR_I420, COLOR_YUV2BGR_NV12, COLOR_YUV2BGR_UYVY, COLOR_YUV2BGR_YUY2,
    COLOR_YUV2BGR_YVYU,
};
use opencv::objdetect::CascadeClassifier;
use opencv::prelude::*;
use std::ffi::OsString;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use video_capture::media::media_frame::MediaFrameDescription;
use video_capture::media::video::PixelFormat;
use video_capture::{
    camera::CameraManager,
    device::{Device, OutputDevice},
    variant::Variant,
};
use x_media::video::VideoFormat;

fn main() -> eframe::Result {
    env_logger::init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "VideoCapture + OpenCV",
        native_options,
        Box::new(|cc| Ok(Box::new(CameraApp::new(cc)))),
    )
}

fn camera_thread_main(app_state: Arc<Mutex<AppState>>) {
    // Create a default instance of camera manager
    let mut cam_mgr = match CameraManager::default() {
        Ok(cam_mgr) => cam_mgr,
        Err(e) => {
            error!("{:?}", e.to_string());
            return;
        }
    };

    // List all camera devices
    let devices = cam_mgr.list();
    for (index, device) in devices.iter().enumerate() {
        info!(
            "device. index: {}, name: {}, id: {}",
            index,
            device.name(),
            device.id()
        );
    }

    for index in 0..devices.len() {
        info!("Getting formats for device: {}", index);
        let Some(device) = cam_mgr.index_mut(index) else {
            continue;
        };

        let _ = device.set_output_handler(|_| Ok(()));

        if device.start().is_ok() {
            // Get supported formats
            let formats = device.formats();
            if let Ok(formats) = formats {
                if let Some(iter) = formats.array_iter() {
                    for format in iter {
                        if let Variant::UInt32(code) = format["format"] {
                            info!("format code: {:?}", code);
                            info!("format: {:?}", VideoFormat::try_from(code).unwrap());
                        }
                        info!("color-range: {:?}", format["color-range"]);
                        info!("width: {:?}", format["width"]);
                        info!("height: {:?}", format["height"]);
                        info!("frame-rates: {:?}", format["frame-rates"]);
                    }
                }
            }
            let _ = device.stop();
        }
    }

    // Get the first camera
    let device = match cam_mgr.index_mut(0) {
        Some(device) => device,
        None => {
            error!("no camera found");
            return;
        }
    };

    // Set the output handler for the camera
    if let Err(e) = device.set_output_handler({
        let app_state = app_state.clone();
        move |frame| {
            let capture_timestamp = chrono::Utc::now();

            debug!("frame source: {:?}", frame.source);
            debug!("frame desc: {:?}", frame.description());
            debug!("frame timestamp: {:?}", frame.timestamp);

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
                        let mut app_state = app_state.lock().unwrap();

                        let faces = app_state
                            .face_classifier
                            .as_mut()
                            .map(|mut classifier| detect_faces(&mat, &mut classifier).ok())
                            .flatten();

                        //
                        // convert into egui specific types and upload texture into the GPU
                        //

                        let color_image = bgr_mat_to_color_image(&mat);
                        let texture_handle = app_state.context.load_texture(
                            "camera",
                            color_image,
                            egui::TextureOptions::LINEAR,
                        );

                        let result = ProcessingResult {
                            texture: texture_handle,
                            size: Vec2::new(mat.cols() as f32, mat.rows() as f32),
                            timestamp: capture_timestamp,
                            faces: faces
                                .unwrap_or_default()
                                .iter()
                                .map(|r| {
                                    egui::Rect::from_min_size(
                                        Pos2::new(r.x as f32, r.y as f32),
                                        Vec2::new(r.width as f32, r.height as f32),
                                    )
                                })
                                .collect::<Vec<egui::Rect>>(),
                        };

                        app_state.frame_sender.send(result).unwrap();
                    })
                }
            }

            Ok(())
        }
    }) {
        error!("{:?}", e.to_string());
    };

    // Configure the camera
    let mut option = Variant::new_dict();
    option["width"] = 1280.into();
    option["height"] = 720.into();
    option["frame-rate"] = 30.0.into();
    if let Err(e) = device.configure(option) {
        error!("{:?}", e.to_string());
    }

    // Start the camera
    if let Err(e) = device.start() {
        error!("{:?}", e.to_string());
    }

    loop {
        thread::sleep(std::time::Duration::from_millis(100));

        {
            let mut app_state = app_state.lock().unwrap();
            if app_state.shutdown_flag {
                app_state.shutdown_flag = false;
                break;
            }
        }
    }

    // Stop the camera
    if let Err(e) = device.stop() {
        error!("{:?}", e.to_string());
    }
}

fn process_frame<'a, F>(frame: &'a video_capture::media::media_frame::MediaFrame, mut f: F)
where
    F: for<'b> FnMut(Mat),
{
    let mapped = frame.map().unwrap();
    let planes = mapped.planes().unwrap();

    let MediaFrameDescription::Video(vfd) = frame.description() else {
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
        // Add support for I420 (YUV 4:2:0 planar)
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

            // Create mats for all planes
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

            // U and V planes have half the width and height in 4:2:0 subsampling
            let u_mat = unsafe {
                Mat::new_rows_cols_with_data_unsafe(
                    (height / 2) as i32,
                    (width / 2) as i32,
                    CV_8UC1,
                    u_data.as_ptr() as *mut std::ffi::c_void,
                    u_stride as usize,
                )
                .unwrap()
            };

            let v_mat = unsafe {
                Mat::new_rows_cols_with_data_unsafe(
                    (height / 2) as i32,
                    (width / 2) as i32,
                    CV_8UC1,
                    v_data.as_ptr() as *mut std::ffi::c_void,
                    v_stride as usize,
                )
                .unwrap()
            };

            // Create a BGR mat for output
            let mut bgr_mat =
                unsafe { Mat::new_rows_cols(height as i32, width as i32, CV_8UC3) }.unwrap();

            // Merge the planes into a single YUV mat
            let mut yuv_mat =
                unsafe { Mat::new_rows_cols(height as i32 * 3 / 2, width as i32, CV_8UC1) }
                    .unwrap();

            // Copy Y plane (full size)
            let y_roi_rect = opencv::core::Rect::new(0, 0, width as i32, height as i32);
            let y_roi = y_mat.roi(y_roi_rect).unwrap();
            y_roi.copy_to(&mut yuv_mat).unwrap();

            // Copy U plane (quarter size) to the correct position
            let u_roi_rect =
                opencv::core::Rect::new(0, height as i32, (width / 2) as i32, (height / 2) as i32);
            let u_roi = u_mat.roi(u_roi_rect).unwrap();
            u_roi.copy_to(&mut yuv_mat).unwrap();

            // Copy V plane (quarter size) to the correct position
            let v_roi_rect = opencv::core::Rect::new(
                (width / 2) as i32,
                height as i32,
                (width / 2) as i32,
                (height / 2) as i32,
            );
            let v_roi = v_mat.roi(v_roi_rect).unwrap();
            v_roi.copy_to(&mut yuv_mat).unwrap();

            // Convert to BGR
            imgproc::cvt_color(
                &yuv_mat,
                &mut bgr_mat,
                COLOR_YUV2BGR_I420,
                0,
                AlgorithmHint::ALGO_HINT_DEFAULT,
            )
            .unwrap();

            bgr_mat
        }
        _ => {
            panic!(
                "Unsupported pixel format: {:?}. Common formats include YUYV, UYVY, NV12, RGB24, BGR24",
                vfd.format
            );
        }
    };

    f(bgr_mat);
}

fn detect_faces(
    mat: &Mat,
    classifier: &mut CascadeClassifier,
) -> Result<Vector<opencv::core::Rect>, opencv::Error> {
    use opencv::{core, imgproc, prelude::*};

    let mut gray = Mat::default();
    imgproc::cvt_color(
        mat,
        &mut gray,
        imgproc::COLOR_BGR2GRAY,
        0,
        AlgorithmHint::ALGO_HINT_DEFAULT,
    )?;

    let mut faces = Vector::new();
    classifier.detect_multi_scale(
        &gray,
        &mut faces,
        1.1,
        3,
        0,
        core::Size {
            width: 30,
            height: 30,
        },
        core::Size {
            width: 0,
            height: 0,
        },
    )?;

    for f in faces.iter() {
        debug!("Face detected at {:?}", f);
    }

    Ok(faces)
}

fn bgr_mat_to_color_image(bgr_mat: &Mat) -> ColorImage {
    let (width, height) = (bgr_mat.cols() as usize, bgr_mat.rows() as usize);
    let data = bgr_mat.data_bytes().unwrap();

    // Convert to RGBA for egui
    let mut rgba = Vec::with_capacity(width * height * 4);
    for chunk in data.chunks_exact(3) {
        rgba.push(chunk[2]); // R
        rgba.push(chunk[1]); // G
        rgba.push(chunk[0]); // B
        rgba.push(255); // A
    }

    ColorImage::from_rgba_unmultiplied([width, height], &rgba)
}

struct AppState {
    context: egui::Context,
    frame_sender: Sender<ProcessingResult>,
    shutdown_flag: bool,
    face_classifier: Option<CascadeClassifier>,
}

impl AppState {
    fn new(frame_sender: Sender<ProcessingResult>, context: Context) -> Self {
        Self {
            frame_sender,
            context,
            shutdown_flag: false,
            face_classifier: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
struct CameraApp {
    open_cv_path: Option<OsString>,

    #[serde(skip)]
    ui_state: Option<UiState>,
}

struct UiState {
    shared_state: Arc<Mutex<AppState>>,
    latest_result: Option<ProcessingResult>,
    receiver: Receiver<ProcessingResult>,
    capture_handle: Option<JoinHandle<()>>,
}

impl CameraApp {
    pub(crate) fn start_capture(&mut self) {
        let ui_state = self.ui_state.as_mut().unwrap();

        if ui_state.capture_handle.is_some() {
            return;
        }

        {
            let mut app_state = ui_state.shared_state.lock().unwrap();

            if let Some(path) = self.open_cv_path.as_ref() {
                let path = std::path::Path::new(&path)
                    .join("data/haarcascades/haarcascade_frontalface_default.xml");

                app_state.face_classifier = CascadeClassifier::new(path.to_str().unwrap())
                    .inspect_err(|e| error!("{}", e.to_string()))
                    .ok();
            }
        }

        // Start camera thread here as before, passing app_state.clone()
        let handle = thread::spawn({
            let app_state = ui_state.shared_state.clone();
            move || camera_thread_main(app_state)
        });

        ui_state.capture_handle = Some(handle);
    }

    pub(crate) fn stop_capture(&mut self) {
        let ui_state = self.ui_state.as_mut().unwrap();

        if !ui_state.capture_handle.is_some() {
            return;
        }

        {
            let mut app_state = ui_state.shared_state.lock().unwrap();
            app_state.shutdown_flag = true;
        }
        ui_state.capture_handle.take().unwrap().join().unwrap();
    }
}

struct ProcessingResult {
    texture: TextureHandle,
    timestamp: chrono::DateTime<chrono::Utc>,
    faces: Vec<egui::Rect>,
    size: Vec2,
}

impl CameraApp {
    fn new(cc: &CreationContext) -> Self {
        let mut instance: CameraApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        let (frame_sender, receiver) = std::sync::mpsc::channel::<ProcessingResult>();

        let shared_state = Arc::new(Mutex::new(AppState::new(frame_sender, cc.egui_ctx.clone())));
        let ui_state = UiState {
            shared_state,
            latest_result: None,
            receiver,
            capture_handle: None,
        };

        instance.ui_state = Some(ui_state);

        instance
    }
}

impl eframe::App for CameraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::SidePanel::left("side_panel")
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.label("This demo uses 'video-capture' to enumerate cameras and capture video frames.\n\
                         The 'videoio' module from OpenCV is NOT enabled. Thus there is less 'C' baggage (usb drivers, webcam drivers, etc.).\n\
                         Additionally OpenCV itself does not have a way to enumerate cameras, so making a program that can use the same\
                         camera regardless of where it's plugged in or the order in which the OS enumarates this is not possible with just OpenCV.");
                    });

                    ui.separator();

                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        egui::Grid::new("settings").show(ui, |ui| {
                            // TODO camera selection
                            if false {
                                ui.label("Camera:");
                                egui::ComboBox::from_id_salt("camera")
                                    .selected_text({
                                        "First camera"
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.label("First camera");
                                    });
                                ui.end_row();
                            }
                            let ui_state = self.ui_state.as_mut().unwrap();
                            let started = ui_state.capture_handle.is_some();
                            ui.add_enabled_ui(!started, |ui| {
                                if ui.button("Start").clicked() {
                                    self.start_capture();
                                }
                            });
                            ui.add_enabled_ui(started, |ui| {
                                if ui.button("Stop").clicked() {
                                    self.stop_capture();
                                }
                            });
                            ui.end_row();
                        });
                    });
                    ui.separator();
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());

                        ui.label("OpenCV path:");
                        let mut open_cv_path = self.open_cv_path.clone().unwrap_or_default().to_string_lossy().into_owned();

                        if ui.add(egui::TextEdit::singleline(&mut open_cv_path).desired_width(ui.available_width())).changed() {
                            self.open_cv_path = Some(open_cv_path.into());
                        };

                        ui.label("For face detection, specify the OpenCV path above.");
                        ui.label("This program uses the `data/haarcascades/haarcascade_frontalface_default.xml` classifier from the OpenCV data directory.");
                    })
                });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let ui_state = self.ui_state.as_mut().unwrap();
            if let Ok(frame) = ui_state.receiver.try_recv() {
                ui_state.latest_result = Some(frame);
            }

            if let Some(processing_result) = &ui_state.latest_result {
                egui::Frame::NONE.show(ui, |ui| {
                    let image_response = egui::Image::new(&processing_result.texture)
                        .max_size(ui.available_size())
                        .maintain_aspect_ratio(true)
                        .ui(ui);

                    let painter = ui.painter();

                    let image_size = image_response.rect.size();

                    let top_left = image_response.rect.min;

                    let scale = Vec2::new(
                        image_size.x / processing_result.size.x,
                        image_size.y / processing_result.size.y,
                    );

                    for face in &processing_result.faces {
                        // Create rectangles for each face, adjusting the scale image, and offsetting them from the top left of the rendered image.
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(face.min.x * scale.x, face.min.y * scale.y)
                                + top_left.to_vec2(),
                            egui::vec2(face.width() * scale.x, face.height() * scale.y),
                        );
                        painter.rect_stroke(
                            rect,
                            0.0,
                            (2.0, egui::Color32::GREEN),
                            StrokeKind::Inside,
                        );
                    }

                    let overlay_clip_rect = image_response.rect;

                    let mut overlay_ui = ui.new_child(UiBuilder::new().max_rect(overlay_clip_rect));
                    overlay_ui.set_clip_rect(overlay_clip_rect);
                    let _ = egui::Frame::default().show(&mut overlay_ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("{}", processing_result.timestamp))
                                    .monospace()
                                    .color(egui::Color32::GREEN),
                            )
                            .selectable(false),
                        );
                    });
                });
            }
        });

        let ui_state = self.ui_state.as_mut().unwrap();
        if ui_state.capture_handle.is_some() {
            // TODO use request_repaint_after() based on the camera frame rate
            ctx.request_repaint();
        }
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencv::core::Mat;
    use std::num::NonZeroU32;
    use video_capture::media::media_frame::MediaFrame;
    use video_capture::media::video::VideoFrameDescription;

    #[test]
    fn test_process_frame_yuyv() {
        // Create test data for YUYV format
        // YUYV format alternates Y, U, Y, V bytes (Y0 U0 Y1 V0)
        let width = 4; // Small test frame
        let height = 2;
        let stride = width * 2; // 2 bytes per pixel in YUYV

        // Create a simple YUYV pattern
        let data = generate_uniform_yuyv_from_rgb(width, height, 64_u8, 128_u8, 192_u8);
        println!("Data: {:?}", data);

        let frame = MediaFrame::from_data_buffer(
            VideoFrameDescription {
                format: PixelFormat::YUYV,
                color_range: Default::default(),
                color_matrix: Default::default(),
                color_primaries: Default::default(),
                color_transfer_characteristics: Default::default(),
                width: NonZeroU32::new(width).unwrap(),
                height: NonZeroU32::new(height).unwrap(),
                rotation: Default::default(),
                origin: Default::default(),
                transparent: false,
                extra_alpha: false,
                crop_left: 0,
                crop_top: 0,
                crop_right: 0,
                crop_bottom: 0,
            },
            data.as_slice(),
        )
        .unwrap();

        // This will be the resulting BGR matrix from processing
        let mut processed_mat: Option<Mat> = None;

        process_frame(&frame, |mat| {
            // Capture the processed matrix
            processed_mat = Some(mat);
        });

        // Verify the result
        let mat = processed_mat.unwrap();

        // Check dimensions
        assert_eq!(mat.rows(), height as i32);
        assert_eq!(mat.cols(), width as i32);

        // Check type - should be BGR
        assert_eq!(mat.channels(), 3); // BGR has 3 channels

        // Check content (basic check)
        let mut avg_b = 0.0;
        let mut avg_g = 0.0;
        let mut avg_r = 0.0;

        for i in 0..height as i32 {
            for j in 0..width as i32 {
                let pixel = mat.at_2d::<opencv::core::Vec3b>(i, j).unwrap();
                avg_b += pixel[0] as f64;
                avg_g += pixel[1] as f64;
                avg_r += pixel[2] as f64;
            }
        }

        let total_pixels = width * height;
        avg_b /= total_pixels as f64;
        avg_g /= total_pixels as f64;
        avg_r /= total_pixels as f64;

        println!("BGR avg: R: {}, G: {}, B: {}", avg_r, avg_g, avg_b);
        // Allow some margin for conversion differences
        assert!((avg_r - 64.0).abs() < 10.0);
        assert!((avg_g - 128.0).abs() < 10.0);
        assert!((avg_b - 192.0).abs() < 10.0);
    }

    fn generate_uniform_yuyv_from_rgb(width: u32, height: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
        let mut yuyv_data = Vec::with_capacity((width * height * 2) as usize);

        // Convert RGB to YUV
        let (y, u, v) = rgb_to_yuv_itu(r, g, b);

        // Generate uniform YUYV data
        for _ in 0..height {
            for _ in (0..width).step_by(2) {
                // YUYV format: [Y0, U, Y1, V] for two pixels
                // Since all pixels are the same, Y0 = Y1 = y
                yuyv_data.extend_from_slice(&[y, u, y, v]);
            }
        }

        yuyv_data
    }

    fn rgb_to_yuv_itu(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        let r = r as f32;
        let g = g as f32;
        let b = b as f32;

        // BT.601
        let y = 0.299 * r + 0.587 * g + 0.114 * b;
        let u = -0.169 * r - 0.331 * g + 0.5 * b + 128.0;
        let v = 0.5 * r - 0.419 * g - 0.081 * b + 128.0;

        (
            y.clamp(16.0, 235.0) as u8,
            u.clamp(16.0, 240.0) as u8,
            v.clamp(16.0, 240.0) as u8,
        )
    }
}
