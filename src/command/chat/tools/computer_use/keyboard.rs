use std::thread;
use std::time::Duration;

use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

use super::error::AicError;
use super::keymap::{is_modifier, resolve_key, resolve_modifier};

fn event_source() -> Result<CGEventSource, AicError> {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| AicError::EventCreationFailed("failed to create event source".into()))
}

fn post_key(keycode: u16, down: bool, flags: CGEventFlags) -> Result<(), AicError> {
    let source = event_source()?;
    let event = CGEvent::new_keyboard_event(source, keycode, down)
        .map_err(|_| AicError::EventCreationFailed("failed to create keyboard event".into()))?;
    if flags != CGEventFlags::CGEventFlagNull {
        event.set_flags(flags);
    }
    event.post(CGEventTapLocation::HID);
    Ok(())
}

/// Press and release a single key.
pub fn press_key(key: &str) -> Result<(), AicError> {
    let keycode = resolve_key(key)?;
    post_key(keycode, true, CGEventFlags::CGEventFlagNull)?;
    thread::sleep(Duration::from_millis(10));
    post_key(keycode, false, CGEventFlags::CGEventFlagNull)?;
    Ok(())
}

/// Press a key combination. `keys` should have modifiers first, main key last.
/// e.g. ["cmd", "shift", "s"]
pub fn key_combo(keys: &[String]) -> Result<(), AicError> {
    if keys.is_empty() {
        return Err(AicError::EventCreationFailed(
            "combo requires at least one key".into(),
        ));
    }

    // Split into modifiers and the final key
    let mut flags = CGEventFlags::CGEventFlagNull;
    let mut main_key = None;

    for (i, k) in keys.iter().enumerate() {
        if i == keys.len() - 1 && !is_modifier(k) {
            main_key = Some(k.as_str());
        } else if is_modifier(k) {
            flags |= resolve_modifier(k)?;
        } else {
            // Non-modifier before the last position — treat last as main key
            main_key = Some(keys.last().unwrap().as_str());
            // All others before it that are modifiers already handled
            break;
        }
    }

    let main_key = main_key.ok_or_else(|| {
        AicError::EventCreationFailed("combo requires a non-modifier key as last argument".into())
    })?;

    let keycode = resolve_key(main_key)?;
    post_key(keycode, true, flags)?;
    thread::sleep(Duration::from_millis(10));
    post_key(keycode, false, flags)?;
    Ok(())
}

/// Type a string of text character by character.
pub fn type_text(text: &str, delay_ms: u64) -> Result<(), AicError> {
    let source = event_source()?;
    for ch in text.chars() {
        // Create a dummy keyboard event and set the Unicode string on it
        let event = CGEvent::new_keyboard_event(source.clone(), 0, true)
            .map_err(|_| AicError::EventCreationFailed("failed to create typing event".into()))?;
        let buf = [ch as u16];
        event.set_string_from_utf16_unchecked(&buf);
        event.post(CGEventTapLocation::HID);

        let event_up = CGEvent::new_keyboard_event(source.clone(), 0, false)
            .map_err(|_| AicError::EventCreationFailed("failed to create typing event".into()))?;
        event_up.post(CGEventTapLocation::HID);

        thread::sleep(Duration::from_millis(delay_ms));
    }
    Ok(())
}
