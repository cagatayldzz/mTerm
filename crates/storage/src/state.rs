use crate::data_dir;
use std::{
  fs, io,
  path::{Path, PathBuf},
};

/// One tab: its split layout and the directory each pane was in, in pane order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SavedTab {
  pub layout: String,
  pub directories: Vec<PathBuf>,
  /// The position of the focused pane in pane order.
  pub focused: usize,
  pub pinned: bool,
  /// Whether each pane is pinned, in pane order.
  pub pinned_panes: Vec<bool>,
  /// A name given by the user; `None` follows the focused pane's title.
  pub title: Option<String>,
}

/// Where a window was on screen, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SavedBounds {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

/// One window and its tabs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SavedWindow {
  pub tabs: Vec<SavedTab>,
  pub active: usize,
  /// Where the window was; `None` when it was not recorded.
  pub bounds: Option<SavedBounds>,
}

/// What was open when mTerm last ran, so the next launch can continue from it.
/// Running processes and terminal output are not part of it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SavedState {
  pub windows: Vec<SavedWindow>,
}

impl SavedState {
  pub fn default_path() -> PathBuf {
    data_dir().join("state.json")
  }

  /// Loads the saved state; a missing or unreadable file yields an empty state.
  pub fn load(path: &Path) -> Self {
    let Ok(contents) = fs::read(path) else { return Self::default() };
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&contents) else { return Self::default() };
    let windows = json.get("windows").and_then(|value| value.as_array()).map(Vec::as_slice).unwrap_or_default();
    Self { windows: windows.iter().filter_map(Self::window_from_json).collect() }
  }

  pub fn save(&self, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }
    let windows: Vec<_> = self
      .windows
      .iter()
      .map(|window| {
        let tabs: Vec<_> = window
          .tabs
          .iter()
          .map(|tab| serde_json::json!({ "layout": tab.layout, "directories": tab.directories, "focused": tab.focused, "pinned": tab.pinned, "pinned_panes": tab.pinned_panes, "title": tab.title }))
          .collect();
        serde_json::json!({
          "tabs": tabs,
          "active": window.active,
          "bounds": window.bounds.map(|b| serde_json::json!({ "x": b.x, "y": b.y, "width": b.width, "height": b.height })),
        })
      })
      .collect();
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec(&serde_json::json!({ "windows": windows }))?)?;
    fs::rename(temporary, path)
  }

  fn window_from_json(json: &serde_json::Value) -> Option<SavedWindow> {
    let tabs: Vec<_> = json.get("tabs")?.as_array()?.iter().filter_map(Self::tab_from_json).collect();
    if tabs.is_empty() {
      return None;
    }
    let active = json.get("active").and_then(|value| value.as_u64()).unwrap_or(0) as usize;
    let bounds = json.get("bounds").and_then(|bounds| {
      let number = |key: &str| bounds.get(key).and_then(|value| value.as_f64()).map(|value| value as f32);
      let (x, y, width, height) = (number("x")?, number("y")?, number("width")?, number("height")?);
      (width > 0. && height > 0.).then_some(SavedBounds { x, y, width, height })
    });
    Some(SavedWindow { active: active.min(tabs.len() - 1), tabs, bounds })
  }

  fn tab_from_json(json: &serde_json::Value) -> Option<SavedTab> {
    let layout = json.get("layout")?.as_str()?.to_owned();
    let directories = json.get("directories")?.as_array()?.iter().filter_map(|value| value.as_str().map(PathBuf::from)).collect();
    let focused = json.get("focused").and_then(|value| value.as_u64()).unwrap_or(0) as usize;
    let pinned = json.get("pinned").and_then(|value| value.as_bool()).unwrap_or(false);
    let pinned_panes = json
      .get("pinned_panes")
      .and_then(|value| value.as_array())
      .map(|values| values.iter().map(|value| value.as_bool().unwrap_or(false)).collect())
      .unwrap_or_default();
    let title = json.get("title").and_then(|value| value.as_str()).map(str::to_owned);
    Some(SavedTab { layout, directories, focused, pinned, pinned_panes, title })
  }
}
