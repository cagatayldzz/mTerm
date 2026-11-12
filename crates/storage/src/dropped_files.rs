use std::{
  fs::{self, OpenOptions},
  io,
  path::{Path, PathBuf},
  sync::atomic::{AtomicU64, Ordering},
};

static NEXT_DROP: AtomicU64 = AtomicU64::new(0);

/// Copies the transient files macOS hands over while dragging (screenshot thumbnails, previews)
/// out of their protected `TemporaryItems` directory. macOS removes those when the drag ends, so
/// the copies stay available for the program the path is pasted into.
pub struct DroppedPaths(Vec<DroppedPath>);

enum DroppedPath {
  Transient { path: PathBuf, source: fs::File },
  Regular(PathBuf),
}

/// Opens transient files while the originating drag is still active. Every other path must exist
/// now: a path that is already gone would only fail later, silently, inside the receiving program.
pub fn open_dropped_paths(paths: &[PathBuf]) -> io::Result<DroppedPaths> {
  paths
    .iter()
    .map(|path| {
      if is_transient(path) {
        let source = fs::File::open(path).map_err(|error| with_path(path, error))?;
        Ok(DroppedPath::Transient { path: path.clone(), source })
      } else {
        fs::metadata(path).map_err(|error| with_path(path, error))?;
        Ok(DroppedPath::Regular(path.clone()))
      }
    })
    .collect::<io::Result<Vec<_>>>()
    .map(DroppedPaths)
}

impl DroppedPaths {
  /// Copies opened transient files to persistent temporary files.
  pub fn prepare(self) -> io::Result<Vec<PathBuf>> {
    self
      .0
      .into_iter()
      .map(|path| match path {
        DroppedPath::Transient { path, source } => copy_transient(&path, source),
        DroppedPath::Regular(path) => Ok(path),
      })
      .collect()
  }
}

pub fn prepare_dropped_paths(paths: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
  open_dropped_paths(paths)?.prepare()
}

fn is_transient(path: &Path) -> bool {
  path.components().any(|component| component.as_os_str() == "TemporaryItems")
}

fn with_path(path: &Path, error: io::Error) -> io::Error {
  let name = path.file_name().unwrap_or(path.as_os_str()).to_string_lossy();
  io::Error::new(error.kind(), format!("{name}: {error}"))
}

fn copy_transient(path: &Path, mut source: fs::File) -> io::Result<PathBuf> {
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
        // A thumbnail can be handed over before macOS finished writing it; an empty copy would
        // paste a path to an image that does not open.
        match io::copy(&mut source, &mut output) {
          Ok(0) => {
            let _ = fs::remove_file(&destination);
            return Err(with_path(path, io::Error::new(io::ErrorKind::UnexpectedEof, "the file is empty")));
          }
          Ok(_) => return Ok(destination),
          Err(error) => {
            let _ = fs::remove_file(&destination);
            return Err(with_path(path, error));
          }
        }
      }
      Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
      Err(error) => return Err(error),
    }
  }
}
