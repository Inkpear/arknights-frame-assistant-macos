use std::sync::LazyLock;

use cgevents::{EventSource, MouseButton, MouseEvent, Point, SourceState, TapLocation};

/// A single process-wide `CGEventSource`, created once and reused for every
/// synthesised event.
///
/// `CGEventSourceCreate` leaks internal CoreGraphics/HID state on every call
/// and that memory is never reclaimed (not via `Drop`, not via an autorelease
/// pool). `cgevents`' `MouseEvent::post` builds a fresh source per call, so
/// posting on the hot path grows RSS unbounded. Building events against one
/// shared source keeps memory flat.
static SHARED_SOURCE: LazyLock<Option<EventSource>> =
    LazyLock::new(|| match EventSource::new(SourceState::Private) {
        Ok(source) => Some(source),
        Err(e) => {
            log::error!("Failed to create shared CGEventSource: {e}");
            None
        }
    });

fn shared_source() -> anyhow::Result<&'static EventSource> {
    SHARED_SOURCE
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("shared CGEventSource is unavailable"))
}

/// Move the mouse cursor to the specified (x, y) coordinates.
pub fn move_to(x: f64, y: f64) -> anyhow::Result<()> {
    let source = shared_source()?;
    MouseEvent::move_to(Point::new(x, y))
        .build(source)?
        .post(TapLocation::Session);
    Ok(())
}

/// Simulate a left mouse click at the specified (x, y) coordinates.
pub fn left_click(x: f64, y: f64) -> anyhow::Result<()> {
    let source = shared_source()?;
    MouseEvent::button_down(Point::new(x, y), MouseButton::Left)
        .build(source)?
        .post(TapLocation::Session);
    MouseEvent::button_up(Point::new(x, y), MouseButton::Left)
        .build(source)?
        .post(TapLocation::Session);
    Ok(())
}
