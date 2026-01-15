//! Visual transitions for the greeter
//!
//! Provides smooth fade effects when starting a session.

use std::time::{Duration, Instant};

/// Fade-out transition for session start
pub struct FadeOutTransition {
    start_time: Instant,
    duration: Duration,
}

impl FadeOutTransition {
    /// Create a new fade-out transition
    pub fn new(duration_ms: u64) -> Self {
        Self {
            start_time: Instant::now(),
            duration: Duration::from_millis(duration_ms),
        }
    }

    /// Get current opacity (1.0 -> 0.0)
    pub fn opacity(&self) -> f64 {
        let elapsed = self.start_time.elapsed();
        if elapsed >= self.duration {
            0.0
        } else {
            // Ease-out curve for smoother feel
            let progress = elapsed.as_secs_f64() / self.duration.as_secs_f64();
            1.0 - ease_out_quad(progress)
        }
    }

    /// Check if transition is complete
    pub fn is_complete(&self) -> bool {
        self.start_time.elapsed() >= self.duration
    }
}

/// Quadratic ease-out function for smooth deceleration
fn ease_out_quad(t: f64) -> f64 {
    1.0 - (1.0 - t) * (1.0 - t)
}

/// Render UI elements with fade effect
pub fn render_with_fade(
    ctx: &cairo::Context,
    opacity: f64,
    render_fn: impl FnOnce(&cairo::Context) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    if opacity >= 1.0 {
        // Full opacity, render normally
        render_fn(ctx)?;
    } else if opacity > 0.0 {
        // Partial opacity, use group
        ctx.push_group();
        render_fn(ctx)?;
        ctx.pop_group_to_source()?;
        ctx.paint_with_alpha(opacity)?;
    }
    // opacity <= 0.0: don't render anything

    Ok(())
}
