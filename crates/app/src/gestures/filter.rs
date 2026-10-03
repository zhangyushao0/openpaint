//! The navigation filter: two-finger motion becomes a view change only past dead zones that
//! keep pan, zoom and rotation apart. From the cumulative motion since the pair formed, the
//! component that first passes its primary threshold by the largest share leads; the others
//! join only past their larger secondary thresholds. A component's dead zone is taken off when
//! it starts, so the view never jumps.

use super::pair::PairDelta;
use super::{scalar_after_dead_zone, vector_after_dead_zone};
use crate::view::ViewGesture;

/// Points of pan, |ln zoom| and radians a component needs to lead.
const PAN_PRIMARY: f32 = 8.0;
const ZOOM_PRIMARY: f32 = 0.04;
const ROTATION_PRIMARY: f32 = 0.14;
/// And to join a gesture another component leads.
const PAN_SECONDARY: f32 = 18.0;
const ZOOM_SECONDARY: f32 = 0.10;
const ROTATION_SECONDARY: f32 = 0.24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Primary {
    Pan,
    Zoom,
    Rotate,
}

#[derive(Debug, Default)]
pub(super) struct NavigationFilter {
    active: bool,
    /// Since the pair formed: pixels, ln zoom, radians.
    translation: [f32; 2],
    log_zoom: f32,
    rotation: f32,
    primary: Option<Primary>,
    pan_started: bool,
    zoom_started: bool,
    rotation_started: bool,
}

impl NavigationFilter {
    /// True from the pair's first frame until it ends.
    pub(super) const fn active(&self) -> bool {
        self.active
    }

    /// The view change for this frame's pair motion; `None` ends the gesture and resets.
    pub(super) fn update(
        &mut self,
        delta: Option<PairDelta>,
        pixels_per_point: f32,
    ) -> Option<ViewGesture> {
        let Some(delta) = delta else {
            *self = Self::default();
            return None;
        };
        self.active = true;
        self.translation[0] += delta.translation[0];
        self.translation[1] += delta.translation[1];
        self.log_zoom += delta.zoom.max(0.001).ln();
        self.rotation += delta.rotation;
        let pan_primary = PAN_PRIMARY * pixels_per_point;
        let pan_length = self.translation[0].hypot(self.translation[1]);
        if self.primary.is_none() {
            let candidates = [
                (pan_length / pan_primary, Primary::Pan),
                (self.log_zoom.abs() / ZOOM_PRIMARY, Primary::Zoom),
                (self.rotation.abs() / ROTATION_PRIMARY, Primary::Rotate),
            ];
            self.primary = candidates
                .into_iter()
                .filter(|(score, _)| *score >= 1.0)
                .max_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, primary)| primary);
        }
        let primary = self.primary?;
        let threshold = |component, first: f32, second: f32| {
            if primary == component { first } else { second }
        };
        let mut translation = [0.0; 2];
        let pan_threshold = threshold(Primary::Pan, pan_primary, PAN_SECONDARY * pixels_per_point);
        if self.pan_started {
            translation = delta.translation;
        } else if pan_length >= pan_threshold {
            self.pan_started = true;
            translation = vector_after_dead_zone(self.translation, pan_threshold);
        }
        let mut zoom = 1.0;
        let zoom_threshold = threshold(Primary::Zoom, ZOOM_PRIMARY, ZOOM_SECONDARY);
        if self.zoom_started {
            zoom = delta.zoom;
        } else if self.log_zoom.abs() >= zoom_threshold {
            self.zoom_started = true;
            zoom = scalar_after_dead_zone(self.log_zoom, zoom_threshold).exp();
        }
        let mut rotation = 0.0;
        let turn_threshold = threshold(Primary::Rotate, ROTATION_PRIMARY, ROTATION_SECONDARY);
        if self.rotation_started {
            rotation = delta.rotation;
        } else if self.rotation.abs() >= turn_threshold {
            self.rotation_started = true;
            rotation = scalar_after_dead_zone(self.rotation, turn_threshold);
        }
        let still = translation == [0.0; 2] && (zoom - 1.0).abs() < f32::EPSILON && rotation == 0.0;
        (!still).then_some(ViewGesture {
            pivot: delta.center,
            translation,
            zoom,
            rotation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta(translation: [f32; 2], zoom: f32, rotation: f32) -> Option<PairDelta> {
        Some(PairDelta {
            center: [100.0, 100.0],
            translation,
            zoom,
            rotation,
        })
    }

    #[test]
    fn two_fingers_at_rest_do_not_move_the_canvas() {
        let mut filter = NavigationFilter::default();
        assert!(filter.update(delta([0.0; 2], 1.0, 0.0), 1.0).is_none());
        assert!(filter.update(delta([4.0, 0.0], 1.005, 0.0), 1.0).is_none());
        let moved = filter
            .update(delta([6.0, 0.0], 1.005, 0.0), 1.0)
            .expect("the cumulative pan passes its dead zone");
        assert_eq!(moved.translation, [2.0, 0.0]);
        assert_eq!((moved.zoom, moved.rotation), (1.0, 0.0));
        assert!(filter.update(None, 1.0).is_none());
        assert!(!filter.active());
    }

    #[test]
    fn the_pan_dead_zone_is_in_points() {
        let mut filter = NavigationFilter::default();
        assert!(filter.update(delta([10.0, 0.0], 1.0, 0.0), 2.0).is_none());
        let moved = filter
            .update(delta([10.0, 0.0], 1.0, 0.0), 2.0)
            .expect("past 16 px");
        assert_eq!(moved.translation, [4.0, 0.0]);
    }

    #[test]
    fn a_pinch_rejects_incidental_rotation_until_an_intentional_twist() {
        let mut filter = NavigationFilter::default();
        let zoom = filter
            .update(delta([0.0; 2], 1.05, 0.05), 1.0)
            .expect("a deliberate pinch leads with zoom");
        assert!(zoom.zoom > 1.0);
        assert_eq!(zoom.rotation, 0.0);
        for _ in 0..3 {
            let update = filter.update(delta([0.0; 2], 1.0, 0.05), 1.0);
            assert!(update.is_none(), "an incidental twist stays filtered");
        }
        let turned = filter
            .update(delta([0.0; 2], 1.0, 0.05), 1.0)
            .expect("a deliberate cumulative twist joins");
        assert!(turned.rotation > 0.0 && turned.rotation < 0.02);
    }
}
