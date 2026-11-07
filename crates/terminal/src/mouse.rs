use crate::keys::Modifiers;

/// The mouse button (or wheel direction) of a mouse report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
  Left,
  Middle,
  Right,
  /// Motion with no button held.
  None,
  WheelUp,
  WheelDown,
}

/// What the mouse did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseAction {
  Press,
  Release,
  /// Motion with a button held.
  Drag,
  /// Motion with no button held.
  Move,
}

/// A mouse event in zero-based terminal cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseEvent {
  pub button: MouseButton,
  pub action: MouseAction,
  pub column: usize,
  pub line: usize,
  pub modifiers: Modifiers,
}

/// Encodes a mouse event for programs that asked for mouse reporting.
///
/// `sgr` selects the SGR encoding (`CSI < … M`), which has no coordinate limit;
/// the legacy encoding cannot address cells past column or line 222 and returns
/// `None` there.
pub fn encode_mouse(event: MouseEvent, sgr: bool) -> Option<Vec<u8>> {
  let MouseEvent { button, action, column, line, modifiers } = event;
  let mut code: u32 = match button {
    MouseButton::Left => 0,
    MouseButton::Middle => 1,
    MouseButton::Right => 2,
    MouseButton::None => 3,
    MouseButton::WheelUp => 64,
    MouseButton::WheelDown => 65,
  };
  if sgr {
    if matches!(action, MouseAction::Drag | MouseAction::Move) {
      code += 32;
    }
  } else if action == MouseAction::Release {
    // The legacy encoding cannot say which button was released.
    code = 3;
  } else if matches!(action, MouseAction::Drag | MouseAction::Move) {
    code += 32;
  }
  code += 4 * modifiers.shift as u32 + 8 * modifiers.alt as u32 + 16 * modifiers.control as u32;
  if sgr {
    let last = if action == MouseAction::Release { 'm' } else { 'M' };
    return Some(format!("\x1b[<{code};{};{}{last}", column + 1, line + 1).into_bytes());
  }
  if column >= 223 || line >= 223 {
    return None;
  }
  Some(vec![0x1b, b'[', b'M', 32 + code as u8, 33 + column as u8, 33 + line as u8])
}
