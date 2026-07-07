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

/// Get the current cursor position as a tuple of (x, y) coordinates.
pub fn get_cursor_position() -> anyhow::Result<(f64, f64)> {
    let pos = cgevents::Event::new(None)?.location();
    Ok((pos.x, pos.y))
}

/// Release the left mouse button at the current cursor position.
pub fn _release_mouse_left_buttons() -> anyhow::Result<()> {
    let (x, y) = get_cursor_position()?;
    MouseEvent::button_up(Point::new(x, y), cgevents::MouseButton::Left)
        .post(TapLocation::Session)?;
    Ok(())
}
