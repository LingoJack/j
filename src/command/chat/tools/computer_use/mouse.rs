use std::thread;
use std::time::Duration;

use core_graphics::event::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;

// FFI for scroll wheel events (not exposed by core-graphics 0.24)
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreateScrollWheelEvent(
        source: *const std::ffi::c_void,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
        wheel2: i32,
    ) -> *mut std::ffi::c_void;
    fn CGEventPost(tap: u32, event: *mut std::ffi::c_void);
    fn CFRelease(cf: *mut std::ffi::c_void);
}

use super::error::AicError;

fn event_source() -> Result<CGEventSource, AicError> {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| AicError::EventCreationFailed("failed to create event source".into()))
}

fn mouse_event(
    event_type: CGEventType,
    point: CGPoint,
    button: CGMouseButton,
) -> Result<CGEvent, AicError> {
    let source = event_source()?;
    CGEvent::new_mouse_event(source, event_type, point, button)
        .map_err(|_| AicError::EventCreationFailed("failed to create mouse event".into()))
}

/// Move the cursor to an absolute position.
pub fn move_to(x: f64, y: f64) -> Result<(), AicError> {
    let point = CGPoint::new(x, y);
    let event = mouse_event(CGEventType::MouseMoved, point, CGMouseButton::Left)?;
    event.post(CGEventTapLocation::HID);
    Ok(())
}

/// Left-click at position.
pub fn click(x: f64, y: f64) -> Result<(), AicError> {
    let point = CGPoint::new(x, y);

    let down = mouse_event(CGEventType::LeftMouseDown, point, CGMouseButton::Left)?;
    let up = mouse_event(CGEventType::LeftMouseUp, point, CGMouseButton::Left)?;

    down.post(CGEventTapLocation::HID);
    thread::sleep(Duration::from_millis(10));
    up.post(CGEventTapLocation::HID);
    Ok(())
}

/// Double-click at position.
pub fn double_click(x: f64, y: f64) -> Result<(), AicError> {
    let point = CGPoint::new(x, y);

    // First click
    let down1 = mouse_event(CGEventType::LeftMouseDown, point, CGMouseButton::Left)?;
    down1.set_integer_value_field(1, 1); // kCGMouseEventClickState = 1
    let up1 = mouse_event(CGEventType::LeftMouseUp, point, CGMouseButton::Left)?;
    up1.set_integer_value_field(1, 1);

    // Second click
    let down2 = mouse_event(CGEventType::LeftMouseDown, point, CGMouseButton::Left)?;
    down2.set_integer_value_field(1, 2); // click count = 2
    let up2 = mouse_event(CGEventType::LeftMouseUp, point, CGMouseButton::Left)?;
    up2.set_integer_value_field(1, 2);

    down1.post(CGEventTapLocation::HID);
    thread::sleep(Duration::from_millis(10));
    up1.post(CGEventTapLocation::HID);
    thread::sleep(Duration::from_millis(50));
    down2.post(CGEventTapLocation::HID);
    thread::sleep(Duration::from_millis(10));
    up2.post(CGEventTapLocation::HID);
    Ok(())
}

/// Right-click at position.
pub fn right_click(x: f64, y: f64) -> Result<(), AicError> {
    let point = CGPoint::new(x, y);

    let down = mouse_event(CGEventType::RightMouseDown, point, CGMouseButton::Right)?;
    let up = mouse_event(CGEventType::RightMouseUp, point, CGMouseButton::Right)?;

    down.post(CGEventTapLocation::HID);
    thread::sleep(Duration::from_millis(10));
    up.post(CGEventTapLocation::HID);
    Ok(())
}

/// Drag from one position to another.
pub fn drag(x1: f64, y1: f64, x2: f64, y2: f64, duration_ms: u64) -> Result<(), AicError> {
    let start = CGPoint::new(x1, y1);
    let end = CGPoint::new(x2, y2);

    // Mouse down at start
    let down = mouse_event(CGEventType::LeftMouseDown, start, CGMouseButton::Left)?;
    down.post(CGEventTapLocation::HID);

    // Interpolate drag events
    let steps = 20u64;
    let step_delay = Duration::from_millis(duration_ms / steps);

    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let cx = x1 + (x2 - x1) * t;
        let cy = y1 + (y2 - y1) * t;
        let point = CGPoint::new(cx, cy);
        let drag_event = mouse_event(CGEventType::LeftMouseDragged, point, CGMouseButton::Left)?;
        drag_event.post(CGEventTapLocation::HID);
        thread::sleep(step_delay);
    }

    // Mouse up at end
    let up = mouse_event(CGEventType::LeftMouseUp, end, CGMouseButton::Left)?;
    up.post(CGEventTapLocation::HID);
    Ok(())
}

/// Scroll at an optional position.
pub fn scroll(dx: i32, dy: i32, at: Option<(f64, f64)>) -> Result<(), AicError> {
    if let Some((x, y)) = at {
        move_to(x, y)?;
        thread::sleep(Duration::from_millis(10));
    }

    // SAFETY: CGEventCreateScrollWheelEvent / CGEventPost / CFRelease 均为 macOS CoreGraphics
    // 线程安全 C API，传入参数均为值类型或已验证的非空指针
    unsafe {
        let event = CGEventCreateScrollWheelEvent(
            std::ptr::null(),
            0, // line units
            2, // two wheels
            dy,
            dx,
        );
        if event.is_null() {
            return Err(AicError::EventCreationFailed(
                "failed to create scroll event".into(),
            ));
        }
        CGEventPost(0, event); // 0 = kCGHIDEventTap
        CFRelease(event);
    }
    Ok(())
}
