use std::{
  fs::{self, OpenOptions},
  io,
  path::{Path, PathBuf},
  sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DROP: AtomicU64 = AtomicU64::new(0);

/// Copies macOS screenshot thumbnails out of their protected temporary directory.
/// The copied files remain available after the originating drag has ended.
pub fn prepare_dropped_paths(paths: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
  paths.iter().map(|path| if is_screenshot_thumbnail(path) { copy_screenshot(path) } else { Ok(path.clone()) }).collect()
}

fn is_screenshot_thumbnail(path: &Path) -> bool {
  let Some(parent) = path.parent() else { return false };
  parent.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with("NSIRD_screencaptureui_"))
    && parent.parent().and_then(Path::file_name).is_some_and(|name| name == "TemporaryItems")
}

fn copy_screenshot(path: &Path) -> io::Result<PathBuf> {
  let mut source = fs::File::open(path)?;
  let directory = std::env::temp_dir().join("mterm-dropped-images");
  fs::create_dir_all(&directory)?;
  let extension = path.extension().and_then(|extension| extension.to_str()).filter(|extension| extension.chars().all(|c| c.is_ascii_alphanumeric()));
  loop {
    let number = NEXT_DROP.fetch_add(1, Ordering::Relaxed);
    let name = format!("screenshot-{}-{number}", std::process::id());
    let destination = directory.join(match extension {
      Some(extension) => format!("{name}.{extension}"),
      None => name,
    });
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
      use std::os::unix::fs::OpenOptionsExt as _;
      options.mode(0o600);
    }
    match options.open(&destination) {
      Ok(mut output) => {
        if let Err(error) = io::copy(&mut source, &mut output) {
          let _ = fs::remove_file(&destination);
          return Err(error);
        }
        return Ok(destination);
      }
      Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
      Err(error) => return Err(error),
    }
  }
}
