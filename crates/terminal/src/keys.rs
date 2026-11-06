/// The modifier keys held with a key press.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
  pub control: bool,
  pub alt: bool,
  pub shift: bool,
  /// The Command key.
  pub command: bool,
}

impl Modifiers {
  /// The xterm modifier parameter (`1 + shift + 2·alt + 4·control`), or `None` when no modifier is held.
  fn xterm(self) -> Option<u8> {
    let value = 1 + self.shift as u8 + 2 * self.alt as u8 + 4 * self.control as u8;
    (value > 1).then_some(value)
  }
}

/// Encodes a key press as the bytes a terminal program expects.
///
/// `key` is the key name (`"a"`, `"enter"`, `"left"`, …) and `text` the text the
/// key types, if any. Option (alt) with text sends the typed text, so characters
/// such as `@` on non-US layouts still work; Option with the arrow and backspace
/// keys moves by word, like iTerm2. Returns `None` for keys that send nothing.
pub fn encode_key(key: &str, text: Option<&str>, mods: Modifiers, app_cursor: bool) -> Option<Vec<u8>> {
  if mods.command {
    return match key {
      "left" => Some(vec![0x01]),
      "right" => Some(vec![0x05]),
      "backspace" => Some(vec![0x15]),
      _ => None,
    };
  }
  if let Some(bytes) = named_key(key, mods, app_cursor) {
    return Some(bytes);
  }
  let mut chars = key.chars();
  let single = match (chars.next(), chars.next()) {
    (Some(c), None) => Some(c),
    _ => None,
  };
  if mods.control {
    return single.and_then(control_byte).map(|byte| vec![byte]);
  }
  if key == "space" {
    return Some(b" ".to_vec());
  }
  match text {
    Some(text) if !text.is_empty() => Some(text.as_bytes().to_vec()),
    _ => single.filter(|_| !mods.alt).map(|c| c.to_string().into_bytes()),
  }
}

/// Wraps pasted text in bracketed-paste markers when the program asked for them.
pub fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
  let text = text.replace("\r\n", "\r").replace('\n', "\r");
  if bracketed {
    format!("\x1b[200~{text}\x1b[201~").into_bytes()
  } else {
    text.into_bytes()
  }
}

/// Formats dropped paths as shell arguments without executing them.
pub fn dropped_paths<'a>(paths: impl IntoIterator<Item = &'a std::path::Path>) -> String {
  paths
    .into_iter()
    .map(|path| {
      let text = path.to_string_lossy();
      let mut escaped = String::new();
      for character in text.chars() {
        if !character.is_ascii_alphanumeric() && !matches!(character, '/' | '.' | '_' | '-') {
          escaped.push('\\');
        }
        escaped.push(character);
      }
      escaped
    })
    .collect::<Vec<_>>()
    .join(" ")
}

fn control_byte(c: char) -> Option<u8> {
  match c.to_ascii_lowercase() {
    letter @ 'a'..='z' => Some(letter as u8 - b'a' + 1),
    '@' | ' ' => Some(0),
    '[' => Some(0x1b),
    '\\' => Some(0x1c),
    ']' => Some(0x1d),
    '^' => Some(0x1e),
    '_' | '/' => Some(0x1f),
    _ => None,
  }
}

fn named_key(key: &str, mods: Modifiers, app_cursor: bool) -> Option<Vec<u8>> {
  let csi = |code: &str, final_byte: char| -> Vec<u8> {
    match mods.xterm() {
      Some(modifier) => format!("\x1b[{code};{modifier}{final_byte}").into_bytes(),
      None => format!("\x1b[{code}{final_byte}").into_bytes(),
    }
  };
  let cursor = |final_byte: char| -> Vec<u8> {
    if mods.xterm().is_none() && app_cursor {
      format!("\x1bO{final_byte}").into_bytes()
    } else {
      csi(if mods.xterm().is_some() { "1" } else { "" }, final_byte)
    }
  };
  let tilde = |code: &str| csi(code, '~');
  Some(match key {
    "enter" => b"\r".to_vec(),
    "tab" if mods.shift => b"\x1b[Z".to_vec(),
    "tab" => b"\t".to_vec(),
    "escape" => b"\x1b".to_vec(),
    "backspace" if mods.alt => b"\x1b\x7f".to_vec(),
    "backspace" => b"\x7f".to_vec(),
    "left" if mods.alt && !mods.control => b"\x1bb".to_vec(),
    "right" if mods.alt && !mods.control => b"\x1bf".to_vec(),
    "up" => cursor('A'),
    "down" => cursor('B'),
    "right" => cursor('C'),
    "left" => cursor('D'),
    "home" => cursor('H'),
    "end" => cursor('F'),
    "delete" => tilde("3"),
    "pageup" => tilde("5"),
    "pagedown" => tilde("6"),
    "insert" => tilde("2"),
    "f1" => function_low('P', mods),
    "f2" => function_low('Q', mods),
    "f3" => function_low('R', mods),
    "f4" => function_low('S', mods),
    "f5" => tilde("15"),
    "f6" => tilde("17"),
    "f7" => tilde("18"),
    "f8" => tilde("19"),
    "f9" => tilde("20"),
    "f10" => tilde("21"),
    "f11" => tilde("23"),
    "f12" => tilde("24"),
    _ => return None,
  })
}

fn function_low(final_byte: char, mods: Modifiers) -> Vec<u8> {
  match mods.xterm() {
    Some(modifier) => format!("\x1b[1;{modifier}{final_byte}").into_bytes(),
    None => format!("\x1bO{final_byte}").into_bytes(),
  }
}
