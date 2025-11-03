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
}

#[derive(Debug, Clone, Copy)]
pub struct FpsSnapshot {
    pub(crate) latest: f32,
    pub(crate) min: f32,
    pub(crate) max: f32,
    pub(crate) avg: f32,
}
