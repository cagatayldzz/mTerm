use mterm_terminal::{encode_mouse, Modifiers, MouseAction, MouseButton, MouseEvent};

fn event(button: MouseButton, action: MouseAction, column: usize, line: usize) -> MouseEvent {
  MouseEvent { button, action, column, line, modifiers: Modifiers::default() }
}

#[test]
fn sgr_reports_use_one_based_cells_and_tell_press_from_release() {
  assert_eq!(encode_mouse(event(MouseButton::Left, MouseAction::Press, 4, 2), true), Some(b"\x1b[<0;5;3M".to_vec()));
  assert_eq!(encode_mouse(event(MouseButton::Left, MouseAction::Release, 4, 2), true), Some(b"\x1b[<0;5;3m".to_vec()));
  assert_eq!(encode_mouse(event(MouseButton::Right, MouseAction::Press, 0, 0), true), Some(b"\x1b[<2;1;1M".to_vec()));
}

#[test]
fn sgr_reports_wheel_motion_and_modifiers() {
  assert_eq!(encode_mouse(event(MouseButton::WheelUp, MouseAction::Press, 0, 0), true), Some(b"\x1b[<64;1;1M".to_vec()));
  assert_eq!(encode_mouse(event(MouseButton::WheelDown, MouseAction::Press, 0, 0), true), Some(b"\x1b[<65;1;1M".to_vec()));
  assert_eq!(encode_mouse(event(MouseButton::Left, MouseAction::Drag, 1, 1), true), Some(b"\x1b[<32;2;2M".to_vec()));
  assert_eq!(encode_mouse(event(MouseButton::None, MouseAction::Move, 1, 1), true), Some(b"\x1b[<35;2;2M".to_vec()));
  let shifted =
    MouseEvent { modifiers: Modifiers { shift: true, control: true, ..Modifiers::default() }, ..event(MouseButton::Left, MouseAction::Press, 0, 0) };
  assert_eq!(encode_mouse(shifted, true), Some(b"\x1b[<20;1;1M".to_vec()));
}

#[test]
fn legacy_reports_offset_by_32_and_stop_at_the_coordinate_limit() {
  assert_eq!(encode_mouse(event(MouseButton::Left, MouseAction::Press, 4, 2), false), Some(vec![0x1b, b'[', b'M', 32, 37, 35]));
  assert_eq!(encode_mouse(event(MouseButton::Right, MouseAction::Release, 4, 2), false), Some(vec![0x1b, b'[', b'M', 35, 37, 35]));
  assert_eq!(encode_mouse(event(MouseButton::Left, MouseAction::Press, 222, 0), false).map(|bytes| bytes[4]), Some(255));
  assert_eq!(encode_mouse(event(MouseButton::Left, MouseAction::Press, 223, 0), false), None);
}
