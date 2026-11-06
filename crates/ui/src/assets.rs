use gpui_kit::{AssetSource, Result, SharedString};
use std::borrow::Cow;

/// The stroke width the interface draws its icons with. Lucide ships them at 2, which looks heavy.
pub const ICON_STROKE_WIDTH: &str = "1";

/// GPUI Kit's icons, redrawn with a thinner stroke.
pub struct Assets;

impl AssetSource for Assets {
  fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
    let Some(bytes) = gpui_kit_assets::Assets.load(path)? else { return Ok(None) };
    if !path.ends_with(".svg") {
      return Ok(Some(bytes));
    }
    Ok(Some(match std::str::from_utf8(&bytes) {
      Ok(svg) => Cow::Owned(with_stroke_width(svg).into_bytes()),
      Err(_) => bytes,
    }))
  }

  fn list(&self, path: &str) -> Result<Vec<SharedString>> {
    gpui_kit_assets::Assets.list(path)
  }
}

/// Replaces the stroke width of an icon drawn at Lucide's default.
fn with_stroke_width(svg: &str) -> String {
  svg.replace("stroke-width=\"2\"", &format!("stroke-width=\"{ICON_STROKE_WIDTH}\""))
}

#[cfg(test)]
#[path = "../tests/assets.rs"]
mod tests;
