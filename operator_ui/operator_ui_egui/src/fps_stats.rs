use std::time::Instant;

pub struct FpsStats {
    history: Vec<f32>,
    max_len: usize,
    last_update: Option<Instant>,
}

impl FpsStats {
    pub fn new(max_len: usize) -> Self {
        Self {
            history: Vec::with_capacity(max_len),
            max_len,
            last_update: None,
        }
    }

    /// Updates the FPS stats given the current time.
    /// Returns None if this is the first frame (cannot compute FPS yet).
    pub fn update(&mut self, now: Instant) -> Option<FpsSnapshot> {
        let latest_fps = if let Some(last) = self.last_update {
            let elapsed = now.duration_since(last).as_secs_f32();
            if elapsed > 0.0 { 1.0 / elapsed } else { return None; }
        } else {
            self.last_update = Some(now);
            return None; // first frame, can't compute FPS yet
        };

        // store in history
        if self.history.len() >= self.max_len {
            self.history.remove(0);
        }
        self.history.push(latest_fps);

        self.last_update = Some(now);

        // compute snapshot
        let min = self.history.iter().copied().fold(f32::INFINITY, f32::min);
        let max = self.history.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let avg = self.history.iter().copied().sum::<f32>() / self.history.len() as f32;

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
    use egui::{Ui, Color32, Response};
    use egui_plot::{Bar, BarChart, Legend, Plot};
    use crate::fps_stats::FpsStats;

    pub fn show_frame_durations(ui: &mut Ui, fps_stats: &FpsStats) -> Response {
        let durations = fps_stats.frame_durations_ms();

        // Map history to egui_plot bars
        let bars: Vec<Bar> = durations
            .iter()
            .enumerate()
            .map(|(i, &duration)| Bar::new(i as f64, duration as f64).width(1.0).fill(Color32::GREEN))
            .collect();

        let chart = BarChart::new("durations", bars)
            .color(Color32::GREEN)
            .width(1.0); // spacing width

        ui.label("Frame durations (ms)");


        Plot::new("Normal Distribution Demo")
            .legend(Legend::default())
            .width(ui.available_width())
            .height(100.0)
            .clamp_grid(true)
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_double_click_reset(false)
            .show(ui, |plot_ui| plot_ui.bar_chart(chart))
            .response
    }
}