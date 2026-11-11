use std::{
  fs::{self, OpenOptions},
  io,
  path::{Path, PathBuf},
  sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DROP: AtomicU64 = AtomicU64::new(0);

/// Copies macOS screenshot thumbnails out of their protected temporary directory.
/// The copied files remain available after the originating drag has ended.
pub struct DroppedPaths(Vec<DroppedPath>);

enum DroppedPath {
  Screenshot { path: PathBuf, source: fs::File },
  Regular(PathBuf),
}

/// Opens temporary screenshots while the originating drag is still active.
pub fn open_dropped_paths(paths: &[PathBuf]) -> io::Result<DroppedPaths> {
  paths
    .iter()
    .map(|path| {
      if is_screenshot_thumbnail(path) {
        Ok(DroppedPath::Screenshot { path: path.clone(), source: fs::File::open(path)? })
      } else {
        Ok(DroppedPath::Regular(path.clone()))
      }
    })
    .collect::<io::Result<Vec<_>>>()
    .map(DroppedPaths)
}

impl DroppedPaths {
  /// Copies opened screenshots to persistent temporary files.
  pub fn prepare(self) -> io::Result<Vec<PathBuf>> {
    self
      .0
      .into_iter()
      .map(|path| match path {
        DroppedPath::Screenshot { path, source } => copy_screenshot(&path, source),
        DroppedPath::Regular(path) => Ok(path),
      })
      .collect()
  }
}

pub fn prepare_dropped_paths(paths: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
  open_dropped_paths(paths)?.prepare()
}

fn is_screenshot_thumbnail(path: &Path) -> bool {
  let Some(parent) = path.parent() else { return false };
  parent.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with("NSIRD_screencaptureui_"))
    && parent.parent().and_then(Path::file_name).is_some_and(|name| name == "TemporaryItems")
}

fn copy_screenshot(path: &Path, mut source: fs::File) -> io::Result<PathBuf> {
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
