use std::collections::VecDeque;
use std::time::Instant;

pub struct FpsStats<const MAX_LEN: usize> {
    history: VecDeque<f32>,
    last_update: Option<Instant>,
}

impl<const MAX_LEN: usize> FpsStats<MAX_LEN> {
    pub fn new() -> Self {
        Self {
            history: VecDeque::from([0_f32; MAX_LEN]),
            last_update: None,
        }
    }

    /// Updates the FPS stats given the current time.
    /// Returns None if this is the first frame (cannot compute FPS yet).
    pub fn update(&mut self, now: Instant) -> Option<FpsSnapshot> {
        let latest_fps = if let Some(last) = self.last_update {
            let elapsed = now.duration_since(last).as_secs_f32();
            if elapsed > 0.0 {
                1.0 / elapsed
            } else {
                return None;
            }
        } else {
            self.last_update = Some(now);
            return None; // first frame, can't compute FPS yet
        };

        // store in history
        self.history.pop_front();
        self.history.push_back(latest_fps);

        self.last_update = Some(now);

        // compute snapshot, ignoring zero fps values
        let (min, max, sum, count) = self
            .history
            .iter()
            .copied()
            .filter(|&fps| fps > 0.0)
            .fold(
                (f32::INFINITY, f32::NEG_INFINITY, 0.0, 0),
                |(min, max, sum, count), fps| (min.min(fps), max.max(fps), sum + fps, count + 1),
            );
        let avg = sum / count as f32;

        Some(FpsSnapshot {
            latest: latest_fps,
            min,
            max,
            avg,
        })
    }

    pub fn frame_durations_ms(&self) -> Vec<f32> {
        self.history
            .iter()
            .map(|&fps| if fps > 0.0 { 1000.0 / fps } else { 0.0 })
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FpsSnapshot {
    pub(crate) latest: f32,
    pub(crate) min: f32,
    pub(crate) max: f32,
    pub(crate) avg: f32,
}

pub mod egui {
    use egui::{Color32, Response, Ui};
    use egui_plot::{Bar, BarChart, Plot};

    use crate::fps_stats::FpsStats;

    pub fn show_frame_durations<const MAX_LEN: usize>(ui: &mut Ui, fps_stats: &FpsStats<MAX_LEN>) -> Response {
        let durations = fps_stats.frame_durations_ms();

        // Map history to egui_plot bars
        let bars: Vec<Bar> = durations
            .iter()
            .enumerate()
            .map(|(i, &duration)| {
                Bar::new(i as f64, duration as f64)
                    .width(1.0)
                    .fill(Color32::GREEN)
            })
            .collect();

        let chart = BarChart::new("durations", bars)
            .color(Color32::GREEN)
            .width(1.0); // spacing width

        ui.label("Frame durations (ms)");

        // NOTE: 1/7.5 = 133ms, so 150 seems a reasonable cap.
        Plot::new("frame_duration_stats")
            .width(ui.available_width())
            .default_y_bounds(0.0, 150.0)
            .height(100.0)
            .show_axes([false, true])
            .clamp_grid(true)
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_axis_zoom_drag(false)
            .allow_double_click_reset(false)
            .show(ui, |plot_ui| plot_ui.bar_chart(chart))
            .response
    }
}
