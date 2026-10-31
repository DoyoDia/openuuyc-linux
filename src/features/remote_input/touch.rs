//! Native multi-touch forwarding state machine.
//!
//! winit reports one contact per event while the wire protocol carries full
//! contact-set snapshots, and its phases map onto the protocol's `kind` with
//! one asymmetry: only the first finger of a gesture opens with kind 1, every
//! later contact change is a kind 2 snapshot the host diffs against its own
//! active set (new ids inject DOWN, vanished ids inject UP).
use std::collections::BTreeMap;

use crate::features::host::input::wire::Point;

// The Windows player is the only caller so far.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Phase {
    Start,
    Move,
    End,
    Cancel,
}

/// One tracked contact: normalized position plus pressure, the only property
/// winit carries besides the id. The wire's radius/angle stay zero.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Contact {
    x: f64,
    y: f64,
    pressure: f64,
}

#[derive(Clone, Default)]
pub(crate) struct Forwarder {
    active: BTreeMap<u32, Contact>,
}

impl Forwarder {
    pub(crate) fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    /// Turns one winit contact event into a wire frame. `position` is the
    /// contact in normalized remote coordinates, or `None` when it falls
    /// outside the video; a `None` return means the event belongs to the
    /// local UI and must not be consumed.
    pub(crate) fn step(
        &mut self,
        phase: Phase,
        id: u32,
        pressure: f64,
        position: Option<(f64, f64)>,
    ) -> Option<(u8, Vec<Point>)> {
        let Some(contact) = position.map(|(x, y)| Contact { x, y, pressure }) else {
            // A new contact on the local UI never joins the remote set.
            if phase == Phase::Start {
                return None;
            }
            // An untracked contact moving or lifting is the local UI's.
            if !self.active.contains_key(&id) {
                return None;
            }
            // Tracked contact off the video: keep its last position for a
            // move, lift it normally on end or cancel.
            return match phase {
                Phase::Move => self.update(id, None, pressure),
                Phase::End => Some(self.lift(id, 3)),
                Phase::Cancel => Some(self.lift(id, 4)),
                Phase::Start => None,
            };
        };
        match phase {
            Phase::Start => {
                let kind = if self.active.is_empty() { 1 } else { 2 };
                self.active.insert(id, contact);
                Some((kind, self.snapshot()))
            }
            // After a flush (mode switch, focus loss) the OS still delivers
            // the gesture's tail: contacts already lifted remotely must not
            // come back through a stray move.
            Phase::Move | Phase::End | Phase::Cancel if !self.active.contains_key(&id) => None,
            Phase::Move => self.update(id, Some(contact), pressure),
            Phase::End => Some(self.lift(id, 3)),
            Phase::Cancel => Some(self.lift(id, 4)),
        }
    }

    /// Lifts everything, for ownership boundaries; `None` when nothing is held.
    pub(crate) fn clear(&mut self) -> Option<(u8, Vec<Point>)> {
        if self.active.is_empty() {
            return None;
        }
        self.active.clear();
        Some((3, Vec::new()))
    }

    fn update(
        &mut self,
        id: u32,
        contact: Option<Contact>,
        pressure: f64,
    ) -> Option<(u8, Vec<Point>)> {
        if let Some(contact) = contact {
            self.active.insert(id, contact);
        } else if let Some(held) = self.active.get_mut(&id) {
            // Slid onto the local UI: the remote finger rests at its last
            // position until it returns or lifts.
            held.pressure = pressure;
        }
        Some((2, self.snapshot()))
    }

    fn lift(&mut self, id: u32, last_kind: u8) -> (u8, Vec<Point>) {
        self.active.remove(&id);
        if self.active.is_empty() {
            (last_kind, Vec::new())
        } else {
            (2, self.snapshot())
        }
    }

    fn snapshot(&self) -> Vec<Point> {
        self.active
            .iter()
            .map(|(id, contact)| Point {
                id: *id,
                x: contact.x,
                y: contact.y,
                radius_x: 0.0,
                radius_y: 0.0,
                angle: 0.0,
                pressure: contact.pressure,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(kind: u8, contacts: &[(u32, f64, f64)]) -> (u8, Vec<Point>) {
        (
            kind,
            contacts
                .iter()
                .map(|(id, x, y)| Point {
                    id: *id,
                    x: *x,
                    y: *y,
                    radius_x: 0.0,
                    radius_y: 0.0,
                    angle: 0.0,
                    pressure: 0.0,
                })
                .collect(),
        )
    }

    #[test]
    fn first_finger_opens_second_joins() {
        let mut f = Forwarder::default();
        assert_eq!(
            f.step(Phase::Start, 2, 0.0, Some((0.25, 0.5))),
            Some(frame(1, &[(2, 0.25, 0.5)]))
        );
        assert_eq!(
            f.step(Phase::Start, 5, 0.0, Some((0.75, 0.5))),
            Some(frame(2, &[(2, 0.25, 0.5), (5, 0.75, 0.5)]))
        );
        assert_eq!(
            f.step(Phase::Move, 2, 0.0, Some((0.3, 0.6))),
            Some(frame(2, &[(2, 0.3, 0.6), (5, 0.75, 0.5)]))
        );
        // One finger lifts, the other stays: a snapshot without it.
        assert_eq!(
            f.step(Phase::End, 2, 0.0, Some((0.3, 0.6))),
            Some(frame(2, &[(5, 0.75, 0.5)]))
        );
        // The last lift reports it for the whole set.
        assert_eq!(f.step(Phase::End, 5, 0.0, None), Some(frame(3, &[])));
    }

    #[test]
    fn last_cancel_reports_cancel() {
        let mut f = Forwarder::default();
        f.step(Phase::Start, 1, 0.0, Some((0.5, 0.5)));
        assert_eq!(f.step(Phase::Cancel, 1, 0.0, None), Some(frame(4, &[])));
    }

    #[test]
    fn local_ui_contacts_never_join() {
        let mut f = Forwarder::default();
        // Starts outside the video: local UI, not consumed.
        assert_eq!(f.step(Phase::Start, 9, 0.0, None), None);
        // Its later events are the local UI's as well.
        assert_eq!(f.step(Phase::Move, 9, 0.0, Some((0.5, 0.5))), None);
        assert_eq!(f.step(Phase::End, 9, 0.0, None), None);
    }

    #[test]
    fn tracked_contact_sliding_onto_ui_keeps_last_position() {
        let mut f = Forwarder::default();
        f.step(Phase::Start, 1, 0.0, Some((0.4, 0.4)));
        assert_eq!(
            f.step(Phase::Move, 1, 0.0, None),
            Some(frame(2, &[(1, 0.4, 0.4)]))
        );
        // Back on the video it tracks again.
        assert_eq!(
            f.step(Phase::Move, 1, 0.0, Some((0.6, 0.6))),
            Some(frame(2, &[(1, 0.6, 0.6)]))
        );
    }

    #[test]
    fn flush_suppresses_the_gesture_tail() {
        let mut f = Forwarder::default();
        f.step(Phase::Start, 1, 0.0, Some((0.4, 0.4)));
        assert_eq!(f.clear(), Some(frame(3, &[])));
        assert_eq!(f.clear(), None);
        // A stray move after the flush must not resurrect the contact.
        assert_eq!(f.step(Phase::Move, 1, 0.0, Some((0.5, 0.5))), None);
        assert_eq!(f.step(Phase::End, 1, 0.0, None), None);
    }
}
