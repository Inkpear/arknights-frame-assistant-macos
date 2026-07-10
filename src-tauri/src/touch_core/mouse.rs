use cgevents::{MouseEvent, Point, TapLocation};

/// Move the mouse cursor to the specified (x, y) coordinates.
pub fn move_to(x: f64, y: f64) -> anyhow::Result<()> {
    MouseEvent::move_to(Point::new(x, y)).post(TapLocation::Session)?;
    Ok(())
}

/// Simulate a left mouse click at the specified (x, y) coordinates.
pub fn left_click(x: f64, y: f64) -> anyhow::Result<()> {
    MouseEvent::button_down(Point::new(x, y), cgevents::MouseButton::Left)
        .post(TapLocation::Session)?;
    MouseEvent::button_up(Point::new(x, y), cgevents::MouseButton::Left)
        .post(TapLocation::Session)?;
    Ok(())
}
