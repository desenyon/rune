//! Shared motion engine (S040), shared-element transitions (S041), adaptive
//! rendering (S042). Decorative motion stays restrained. Reduced motion makes
//! every effect instant.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StyleSnapshot {
    pub fg: Option<Rgba>,
    pub bg: Option<Rgba>,
    pub bold: bool,
}

impl Default for StyleSnapshot {
    fn default() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Spring,
}

impl Easing {
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Self::Spring => {
                // Underdamped settle toward 1.
                1.0 - (-6.0 * t).exp() * (t * std::f32::consts::PI * 2.0).cos()
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MotionConfig {
    pub reduced_motion: bool,
}

impl Default for MotionConfig {
    fn default() -> Self {
        Self {
            reduced_motion: false,
        }
    }
}

pub fn duration_for(config: &MotionConfig, normal: Duration) -> Duration {
    if config.reduced_motion {
        Duration::ZERO
    } else {
        normal
    }
}

pub fn progress_for(config: &MotionConfig, t: f32) -> f32 {
    if config.reduced_motion {
        1.0
    } else {
        t.clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderMode {
    /// Event-driven; no timer ticks.
    Static,
    /// Approximately 30 frames per second.
    Ordinary,
    /// Up to approximately 60 frames per second.
    HighFidelity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameBudget {
    pub mode: RenderMode,
    pub target_fps: u32,
}

impl FrameBudget {
    pub fn static_mode() -> Self {
        Self {
            mode: RenderMode::Static,
            target_fps: 0,
        }
    }

    pub fn ordinary() -> Self {
        Self {
            mode: RenderMode::Ordinary,
            target_fps: 30,
        }
    }

    pub fn high_fidelity() -> Self {
        Self {
            mode: RenderMode::HighFidelity,
            target_fps: 60,
        }
    }

    pub fn for_activity(animating: bool, high_fidelity: bool) -> Self {
        if !animating {
            Self::static_mode()
        } else if high_fidelity {
            Self::high_fidelity()
        } else {
            Self::ordinary()
        }
    }

    pub fn frame_time(self) -> Option<Duration> {
        if self.target_fps == 0 {
            None
        } else {
            Some(Duration::from_nanos(
                1_000_000_000 / u64::from(self.target_fps),
            ))
        }
    }
}

/// Hint for buffer-diffing renderers. Static frames should skip terminal writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BufferDiffHint {
    pub dirty: bool,
    pub synchronized: bool,
    pub skip_write: bool,
}

impl BufferDiffHint {
    pub fn after_render(previous_hash: u64, current_hash: u64, synchronized: bool) -> Self {
        let dirty = previous_hash != current_hash;
        Self {
            dirty,
            synchronized,
            skip_write: !dirty,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    Fade,
    Slide,
    Reveal,
    Crossfade,
    Spring,
    Stagger,
    ColorInterpolation,
    GradientSweep,
    CharacterDissolve,
    BorderTrace,
    HighlightPulse,
    SharedElement,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Effect {
    pub kind: EffectKind,
    pub duration: Duration,
    pub easing: Easing,
    pub delay: Duration,
}

impl Effect {
    pub fn new(kind: EffectKind, duration: Duration) -> Self {
        Self {
            kind,
            duration,
            easing: match kind {
                EffectKind::Spring | EffectKind::SharedElement => Easing::Spring,
                EffectKind::Fade | EffectKind::Crossfade => Easing::EaseOut,
                _ => Easing::EaseInOut,
            },
            delay: Duration::ZERO,
        }
    }

    pub fn stagger(index: usize, step: Duration, inner: Effect) -> Self {
        Self {
            kind: EffectKind::Stagger,
            delay: step.saturating_mul(index as u32),
            ..inner
        }
    }

    pub fn duration(&self, config: &MotionConfig) -> Duration {
        duration_for(config, self.duration.saturating_add(self.delay))
    }

    pub fn sample(&self, elapsed: Duration, config: &MotionConfig) -> EffectSample {
        if config.reduced_motion {
            return EffectSample::complete(self.kind);
        }
        if elapsed < self.delay {
            return EffectSample {
                kind: self.kind,
                progress: 0.0,
                opacity: if matches!(self.kind, EffectKind::Fade | EffectKind::Reveal) {
                    0.0
                } else {
                    1.0
                },
                offset: (0.0, 0.0),
                clip: 0.0,
                color_mix: 0.0,
                pulse: 0.0,
                dissolve: 0.0,
                border: 0.0,
                done: false,
            };
        }
        let local = elapsed.saturating_sub(self.delay);
        let total = self.duration.as_secs_f32().max(0.0001);
        let linear = (local.as_secs_f32() / total).clamp(0.0, 1.0);
        let t = self.easing.apply(linear);
        EffectSample {
            kind: self.kind,
            progress: t,
            opacity: match self.kind {
                EffectKind::Fade | EffectKind::Reveal => t,
                EffectKind::Crossfade => t,
                _ => 1.0,
            },
            offset: match self.kind {
                EffectKind::Slide => ((1.0 - t) * 8.0, 0.0),
                _ => (0.0, 0.0),
            },
            clip: match self.kind {
                EffectKind::Reveal | EffectKind::BorderTrace => t,
                _ => 1.0,
            },
            color_mix: match self.kind {
                EffectKind::ColorInterpolation | EffectKind::GradientSweep => t,
                _ => 0.0,
            },
            pulse: match self.kind {
                EffectKind::HighlightPulse => (t * std::f32::consts::PI).sin(),
                _ => 0.0,
            },
            dissolve: match self.kind {
                EffectKind::CharacterDissolve => t,
                _ => 1.0,
            },
            border: match self.kind {
                EffectKind::BorderTrace => t,
                _ => 0.0,
            },
            done: linear >= 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EffectSample {
    pub kind: EffectKind,
    pub progress: f32,
    pub opacity: f32,
    pub offset: (f32, f32),
    pub clip: f32,
    pub color_mix: f32,
    pub pulse: f32,
    pub dissolve: f32,
    pub border: f32,
    pub done: bool,
}

impl EffectSample {
    fn complete(kind: EffectKind) -> Self {
        Self {
            kind,
            progress: 1.0,
            opacity: 1.0,
            offset: (0.0, 0.0),
            clip: 1.0,
            color_mix: 1.0,
            pulse: 0.0,
            dissolve: 1.0,
            border: 1.0,
            done: true,
        }
    }
}

pub fn interpolate_color(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let lerp =
        |x: u8, y: u8| -> u8 { (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8 };
    Rgba {
        r: lerp(a.r, b.r),
        g: lerp(a.g, b.g),
        b: lerp(a.b, b.b),
        a: lerp(a.a, b.a),
    }
}

pub fn gradient_sweep(from: Rgba, to: Rgba, cells: usize, t: f32) -> Vec<Rgba> {
    let t = t.clamp(0.0, 1.0);
    let visible = ((cells as f32) * t).ceil() as usize;
    (0..cells)
        .map(|i| {
            if i >= visible {
                from
            } else if cells <= 1 {
                to
            } else {
                interpolate_color(from, to, i as f32 / (cells - 1) as f32)
            }
        })
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharedElement {
    pub id: String,
    pub old_rect: Rect,
    pub new_rect: Rect,
    pub old_style: StyleSnapshot,
    pub new_style: StyleSnapshot,
    pub progress: f32,
    pub easing: Easing,
}

impl SharedElement {
    pub fn interpolate(&self, config: &MotionConfig) -> (Rect, StyleSnapshot) {
        let t = self.easing.apply(progress_for(config, self.progress));
        let rect = Rect {
            x: lerp(self.old_rect.x, self.new_rect.x, t),
            y: lerp(self.old_rect.y, self.new_rect.y, t),
            width: lerp(self.old_rect.width, self.new_rect.width, t),
            height: lerp(self.old_rect.height, self.new_rect.height, t),
        };
        let style = StyleSnapshot {
            fg: mix_opt(self.old_style.fg, self.new_style.fg, t),
            bg: mix_opt(self.old_style.bg, self.new_style.bg, t),
            bold: if t >= 0.5 {
                self.new_style.bold
            } else {
                self.old_style.bold
            },
        };
        (rect, style)
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn mix_opt(a: Option<Rgba>, b: Option<Rgba>, t: f32) -> Option<Rgba> {
    match (a, b) {
        (Some(a), Some(b)) => Some(interpolate_color(a, b, t)),
        (Some(a), None) if t < 1.0 => Some(a),
        (None, Some(b)) if t > 0.0 => Some(b),
        (None, None) => None,
        (Some(_), None) => None,
        (None, Some(b)) => Some(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduced_motion_zeroes_duration() {
        let config = MotionConfig {
            reduced_motion: true,
        };
        let effect = Effect::new(EffectKind::Fade, Duration::from_millis(240));
        assert_eq!(effect.duration(&config), Duration::ZERO);
        let sample = effect.sample(Duration::from_millis(0), &config);
        assert_eq!(sample.progress, 1.0);
        assert!(sample.done);
    }

    #[test]
    fn ordinary_and_high_fidelity_budgets() {
        assert_eq!(FrameBudget::ordinary().target_fps, 30);
        assert_eq!(FrameBudget::high_fidelity().target_fps, 60);
        assert!(FrameBudget::static_mode().frame_time().is_none());
        assert_eq!(
            FrameBudget::for_activity(false, true).mode,
            RenderMode::Static
        );
    }

    #[test]
    fn shared_element_reaches_new_rect() {
        let el = SharedElement {
            id: "symbol".into(),
            old_rect: Rect::new(0.0, 0.0, 10.0, 1.0),
            new_rect: Rect::new(20.0, 5.0, 30.0, 2.0),
            old_style: StyleSnapshot::default(),
            new_style: StyleSnapshot::default(),
            progress: 1.0,
            easing: Easing::Linear,
        };
        let (rect, _) = el.interpolate(&MotionConfig::default());
        assert_eq!(rect.x, 20.0);
        assert_eq!(rect.width, 30.0);
    }

    #[test]
    fn buffer_diff_skips_identical_frames() {
        let hint = BufferDiffHint::after_render(1, 1, true);
        assert!(hint.skip_write);
        assert!(!hint.dirty);
    }
}
