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
use egui::{ColorImage, Context, Pos2, Rect, RichText, TextureHandle, UiBuilder, Vec2, Widget};
use egui_ltreeview::{Action, TreeView};
use log::{debug, error, info, trace};
use opencv::core::{AlgorithmHint, CV_8UC1, CV_8UC2, CV_8UC3, CV_8UC4, Vector};
use opencv::imgproc;
use opencv::imgproc::{
    COLOR_YUV2BGR_I420, COLOR_YUV2BGR_NV12, COLOR_YUV2BGR_UYVY, COLOR_YUV2BGR_YUY2,
    COLOR_YUV2BGR_YVYU,
};
use opencv::objdetect::CascadeClassifier;
use opencv::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use video_capture::error::DeviceError;
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Resolution {
    width: u32,
    height: u32,
}

impl Into<Vec2> for Resolution {
    fn into(self) -> Vec2 {
        Vec2::new(self.width as f32, self.height as f32)
    }
}

#[derive(Clone, Debug)]
struct CameraEnumerationResult {
    id: String,
    name: String,

    // resolution/format/framerate
    modes: BTreeMap<Resolution, BTreeMap<u32, Vec<f32>>>,
}

fn camera_enumeration_thread_main() -> Result<Vec<CameraEnumerationResult>, DeviceError> {
    let mut cameras: Vec<CameraEnumerationResult> = vec![];

    let mut cam_mgr = CameraManager::default()?;

    let device_count = cam_mgr.list().len();

    for index in 0..device_count {
        info!("Getting formats for device: {}", index);
        let Some(device) = cam_mgr.index_mut(index) else {
            continue;
        };

        // 'device.formats' can't be called until there's an output handler set and the device is started
        let _ = device.set_output_handler(|_| Ok(()));

        if device.start().is_ok() {
            // Get supported formats
            let formats = device.formats();
            if let Ok(formats) = formats {
                if let Some(iter) = formats.array_iter() {
                    let mut enumeration_result = CameraEnumerationResult {
                        id: device.id().to_string(),
                        name: device.name().to_string(),
                        modes: Default::default(),
                    };

                    for format in iter {
                        if let Variant::UInt32(video_format_code) = format["format"] {
                            let Ok(_video_format) = VideoFormat::try_from(video_format_code) else {
                                // ensure we can use it.
                                continue;
                            };

                            let resolution = match (&format["width"], &format["height"]) {
                                (Variant::UInt32(width), Variant::UInt32(height)) => {
                                    Some(Resolution {
                                        width: *width,
                                        height: *height,
                                    })
                                }
                                _ => None,
                            };
                            let Some(resolution) = resolution else {
                                continue;
                            };

                            let frame_rates = if let Variant::Array(rates) = &format["frame-rates"]
                            {
                                rates
                                    .iter()
                                    .filter_map(|it| match it {
                                        Variant::Float(rate) => Some(*rate),
                                        Variant::Double(rate) => Some(*rate as f32),
                                        _ => None,
                                    })
                                    .collect::<Vec<f32>>()
                            } else {
                                continue;
                            };

                            let format_mapping = enumeration_result
                                .modes
                                .entry(resolution)
                                .or_insert(BTreeMap::new());

                            let existing_frame_rates = format_mapping
                                .entry(video_format_code)
                                .or_insert_with(|| Vec::new());

                            existing_frame_rates.extend(frame_rates);
                        }
                    }

                    cameras.push(enumeration_result);
                }
            }
            let _ = device.stop();
        }
    }
    info!("cameras: {:?}", cameras);

    Ok(cameras)
}

#[derive(Debug, Clone)]
struct ModeSelection {
    camera_index: usize,
    id: String,
    resolution: Resolution,
    video_format: VideoFormat,
    frame_rate: f32,
}

fn camera_thread_main(shared_state: Arc<Mutex<CameraSharedState>>, mode_selection: ModeSelection) {
    // Create a default instance of camera manager
    let mut cam_mgr = match CameraManager::default() {
        Ok(cam_mgr) => cam_mgr,
        Err(e) => {
            error!("{:?}", e.to_string());
            return;
        }
    };

    // Get the first camera
    let device = match cam_mgr.index_mut(mode_selection.camera_index) {
        Some(device) => device,
        None => {
            error!("no camera found");
            return;
        }
    };

    // Set the output handler for the camera
    if let Err(e) = device.set_output_handler({
        let app_state = shared_state.clone();
        move |frame| {
            debug!("frame source: {:?}", frame.source);
            debug!("frame desc: {:?}", frame.description());
            debug!("frame timestamp: {:?}", frame.timestamp);

            let capture_timestamp = chrono::Utc::now();
            let capture_instant = Instant::now();
            // TODO using the timestamp from the frame would be better, but need to convert to chrono::DateTime somehow

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
                            instant: capture_instant,
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
    option["width"] = mode_selection.resolution.width.into();
    option["height"] = mode_selection.resolution.height.into();
    option["frame-rate"] = mode_selection.frame_rate.into();
    let format_code: u32 = mode_selection.video_format.into();
    option["format"] = format_code.into();

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
            let mut app_state = shared_state.lock().unwrap();
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

struct CameraSharedState {
    context: egui::Context,
    frame_sender: Sender<ProcessingResult>,
    shutdown_flag: bool,
    face_classifier: Option<CascadeClassifier>,
}

impl CameraSharedState {
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
    enumeration_handle: Option<JoinHandle<Result<Vec<CameraEnumerationResult>, DeviceError>>>,
    enumerated_cameras: Vec<CameraEnumerationResult>,
    cameras: HashMap<String, CameraState>,

    /// when the UI should be refreshed
    ///
    /// this will be the soonest out of all the cameras next expected frame times.
    refresh_at: Option<Instant>,
}

struct CameraState {
    latest_result: Option<ProcessingResult>,
    receiver: Receiver<ProcessingResult>,
    capture_handle: JoinHandle<()>,
    shared_state: Arc<Mutex<CameraSharedState>>,
    mode: ModeSelection,
}

impl CameraApp {
    pub(crate) fn start_enumerating(&mut self) {
        let ui_state = self.ui_state.as_mut().unwrap();
        if ui_state.enumeration_handle.is_some() {
            return;
        }

        let handle = thread::spawn(camera_enumeration_thread_main);
        ui_state.enumeration_handle = Some(handle);
    }
    pub(crate) fn start_capture(&mut self, mode_selection: ModeSelection, context: Context) {
        let ui_state = self.ui_state.as_mut().unwrap();

        if ui_state.cameras.contains_key(&mode_selection.id) {
            return;
        }

        context.request_repaint_after(Duration::from_millis(100));

        let camera_id = mode_selection.id.clone();
        let (frame_sender, receiver) = std::sync::mpsc::channel::<ProcessingResult>();

        let mut camera_shared_state = CameraSharedState::new(frame_sender, context);
        if let Some(path) = self.open_cv_path.as_ref() {
            let path = std::path::Path::new(&path)
                .join("data/haarcascades/haarcascade_frontalface_default.xml");

            camera_shared_state.face_classifier = CascadeClassifier::new(path.to_str().unwrap())
                .inspect_err(|e| error!("{}", e.to_string()))
                .ok();
        }

        let camera_shared_state = Arc::new(Mutex::new(camera_shared_state));

        let handle = thread::spawn({
            let camera_shared_state = camera_shared_state.clone();
            let mode_selection = mode_selection.clone();
            move || camera_thread_main(camera_shared_state, mode_selection)
        });

        let camera_state = CameraState {
            shared_state: camera_shared_state,
            latest_result: None,
            receiver,
            capture_handle: handle,
            mode: mode_selection,
        };

        ui_state.cameras.insert(camera_id, camera_state);
    }

    pub(crate) fn stop_capture(&mut self, camera_id: &String) {
        let ui_state = self.ui_state.as_mut().unwrap();

        if !ui_state.cameras.contains_key(camera_id) {
            return;
        }

        let camera_state = ui_state.cameras.remove(camera_id).unwrap();

        {
            let mut app_state = camera_state.shared_state.lock().unwrap();
            app_state.shutdown_flag = true;
        }
        camera_state.capture_handle.join().unwrap();
    }

    pub(crate) fn stop_all_cameras(&mut self) {
        let ui_state = self.ui_state.as_mut().unwrap();
        let ids = ui_state.cameras.keys().cloned().collect::<Vec<_>>();
        for id in ids {
            self.stop_capture(&id);
        }
    }
}

struct ProcessingResult {
    /// The instant when the frame was captured
    instant: Instant,
    /// The timestamp of the frame
    timestamp: chrono::DateTime<chrono::Utc>,

    texture: TextureHandle,
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

        let ui_state = UiState {
            enumeration_handle: None,
            enumerated_cameras: Default::default(),
            cameras: Default::default(),
            refresh_at: None,
        };

        instance.ui_state = Some(ui_state);

        instance
    }
}

impl eframe::App for CameraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let redraw_instant = Instant::now();
        {
            let ui_state = self.ui_state.as_mut().unwrap();
            if let Some(refresh_at) = ui_state.refresh_at {
                if redraw_instant >= refresh_at {
                    ui_state.refresh_at = None;
                }
            }
        }

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
                            let ui_state = self.ui_state.as_mut().unwrap();
                            let cameras_running = !ui_state.cameras.is_empty();
                            let enumerating = ui_state.enumeration_handle.is_some();
                            let have_enumerated_cameras = !ui_state.enumerated_cameras.is_empty();

                            let can_start_enumeration = !(cameras_running || enumerating);

                            ui.add_enabled_ui(can_start_enumeration, |ui| {
                                if ui.button("Enumerate").clicked() {
                                    self.start_enumerating();
                                }
                            });
                            if enumerating {
                                ui.spinner();
                            }
                            ui.end_row();

                            ui.add_enabled_ui(cameras_running, |ui| {
                                if ui.button("Stop all").clicked() {
                                    self.stop_all_cameras();
                                }
                            });
                            ui.end_row();

                            if have_enumerated_cameras {
                                ui.label("Double-click a framerate to start!");
                            }
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
                    });
                    ui.separator();
                    {
                        let ui_state = self.ui_state.as_mut().unwrap();
                        // FIXME the treeview takes up space after a restart even though the list is empty, so we check
                        //       remove this if statement once a solution is found
                        if !ui_state.enumerated_cameras.is_empty() {
                            let mut modes: HashMap<i32, ModeSelection> = HashMap::new();

                            let (_response, actions) = TreeView::new(ui.make_persistent_id("cameras"))
                                .allow_drag_and_drop(false)
                                .allow_multi_selection(false)
                                .show(ui, |builder| {
                                let mut node_id = 0;
                                for (camera_index, camera) in ui_state.enumerated_cameras.iter().enumerate() {
                                    builder.dir(node_id, format!("{}: {}", camera_index, camera.name.as_str()));
                                    node_id += 1;

                                    for (resolution, modes_to_rates_mapping) in camera.modes.iter() {
                                        builder.dir(node_id, format!("{:?}", resolution));
                                        node_id += 1;

                                        for (mode, rates) in modes_to_rates_mapping.iter() {
                                            let video_format = VideoFormat::try_from(*mode).unwrap();
                                            builder.dir(node_id, format!("{:?}", video_format));
                                            node_id += 1;

                                            for rate in rates.iter() {
                                                builder.leaf(node_id, format!("{}FPS", rate));

                                                modes.insert(node_id, ModeSelection {
                                                    camera_index,
                                                    id: camera.id.clone(),
                                                    resolution: resolution.clone(),
                                                    video_format: video_format.clone(),
                                                    frame_rate: rate.clone()
                                                });

                                                node_id += 1;
                                            }
                                            builder.close_dir();
                                        }
                                        builder.close_dir();
                                    }
                                    builder.close_dir();
                                }
                            });

                            for action in actions {
                                match action {
                                    Action::Activate(mut node) => {
                                        if node.selected.len() == 1 {
                                            let node_id = node.selected.remove(0);
                                            if let Some(mode) = modes.get(&node_id) {
                                                info!("Selected mode {:?}", mode);

                                                let ui_state = self.ui_state.as_mut().unwrap();
                                                let enumerating = ui_state.enumeration_handle.is_some();

                                                if !enumerating {
                                                    let started = ui_state.cameras.contains_key(&mode.id);
                                                    if started {
                                                        self.stop_capture(&mode.id);
                                                    }
                                                    self.start_capture(mode.clone(), ui.ctx().clone());
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                });
        });

        let mut camera_to_close = vec![];

        egui::CentralPanel::default().show(ctx, |ui| {
            let ui_state = self.ui_state.as_mut().unwrap();

            for (camera_id, camera_state) in ui_state.cameras.iter_mut() {
                let Some(camera_enumeration_result) = ui_state
                    .enumerated_cameras
                    .iter()
                    .find(|it| it.id == *camera_id)
                else {
                    continue;
                };

                let size: Vec2 = camera_state.mode.resolution.clone().into();
                let constrain_rect =
                    Rect::from_min_size(ui.max_rect().min, Vec2::splat(f32::INFINITY));

                let mut open = true;
                egui::Window::new(format!(
                    "{} [{} * {}] @ {}FPS",
                    camera_enumeration_result.name.clone(),
                    camera_state.mode.resolution.width,
                    camera_state.mode.resolution.height,
                    camera_state.mode.frame_rate
                ))
                .id(ui.id().with(camera_id))
                .movable(true)
                .open(&mut open)
                .resizable(true)
                .scroll(true)
                .default_size(size)
                .max_size(size)
                .constrain_to(constrain_rect)
                .show(&ctx, |ui| {
                    if let Ok(processing_result) = camera_state.receiver.try_recv() {
                        trace!(
                            "Received frame. Camera: {}, instant: {:?}",
                            camera_state.mode.camera_index, processing_result.instant
                        );
                        camera_state.latest_result = Some(processing_result);
                    }

                    // recalculate the refresh_at

                    let frame_interval =
                        Duration::from_secs_f32(1.0 / camera_state.mode.frame_rate);
                    let next_frame_instant = camera_state
                        .latest_result
                        .as_ref()
                        .map_or(redraw_instant + Duration::from_millis(250), |pr| {
                            pr.instant + frame_interval
                        });

                    match ui_state.refresh_at {
                        None => {
                            trace!(
                                "Camera: {}, refreshing at {:?}",
                                camera_state.mode.camera_index, next_frame_instant
                            );
                            ui_state.refresh_at = Some(next_frame_instant);
                        }
                        Some(scheduled_time) => {
                            if next_frame_instant < scheduled_time {
                                trace!(
                                    "Camera: {}, priority refreshing at {:?}",
                                    camera_state.mode.camera_index, next_frame_instant
                                );
                                ui_state.refresh_at = Some(next_frame_instant);
                            }
                        }
                    }

                    if let Some(processing_result) = &camera_state.latest_result {
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

                            let mut overlay_ui =
                                ui.new_child(UiBuilder::new().max_rect(overlay_clip_rect));
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

                // handling window closing
                if !open {
                    camera_to_close.push(camera_id.clone());
                }
            }
        });

        for camera_id in camera_to_close {
            self.stop_capture(&camera_id);
        }

        let ui_state = self.ui_state.as_mut().unwrap();
        if let Some(handle) = ui_state.enumeration_handle.as_mut() {
            if handle.is_finished() {
                let handle = ui_state.enumeration_handle.take().unwrap();
                if let Ok(Ok(cameras)) = handle.join() {
                    ui_state.enumerated_cameras = cameras;
                }
            }
        }

        if let Some(refresh_at) = ui_state.refresh_at {
            let remaining_time = refresh_at - redraw_instant;
            trace!(
                "Remaining time until refresh: {:?}us, now: {:?}",
                remaining_time.as_micros(),
                redraw_instant
            );

            ctx.request_repaint_after(remaining_time);
        }
    }

    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.stop_all_cameras();
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
