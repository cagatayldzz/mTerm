use std::{
  fs::{self, OpenOptions},
  io::{self, Write as _},
  path::{Path, PathBuf},
  sync::atomic::{AtomicU64, Ordering},
  time::{SystemTime, UNIX_EPOCH},
};

const LOG_FILE: &str = "mterm.log";
const OLD_LOG_FILE: &str = "mterm.log.1";
/// The log is moved aside once it grows past this, so it never fills the disk.
const LOG_LIMIT: u64 = 512 * 1024;
/// Crash reports kept; older ones are deleted.
const CRASH_LIMIT: usize = 10;
/// GitHub rejects very long URLs, so the prefilled issue text is cut to fit.
const MAX_URL_LENGTH: usize = 7000;
const SEEN_SUFFIX: &str = ".seen.txt";

static NEXT_CRASH: AtomicU64 = AtomicU64::new(0);

/// The log and crash reports of mTerm, kept in one folder (`~/Library/Logs/mTerm`).
///
/// Nothing here leaves the machine by itself: the user decides whether to attach a report to an
/// issue.
pub struct Diagnostics {
  dir: PathBuf,
}

impl Default for Diagnostics {
  fn default() -> Self {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    Self::new(home.join("Library").join("Logs").join("mTerm"))
  }
}

impl Diagnostics {
  pub fn new(dir: PathBuf) -> Self {
    Self { dir }
  }

  pub fn dir(&self) -> &Path {
    &self.dir
  }

  /// Appends one line, with the time, to `mterm.log`.
  pub fn log(&self, message: &str) -> io::Result<()> {
    fs::create_dir_all(&self.dir)?;
    let log = self.dir.join(LOG_FILE);
    if fs::metadata(&log).is_ok_and(|metadata| metadata.len() > LOG_LIMIT) {
      fs::rename(&log, self.dir.join(OLD_LOG_FILE))?;
    }
    let mut file = private_options(OpenOptions::new().append(true).create(true)).open(log)?;
    writeln!(file, "{} {}", now(), message.replace('\n', " | "))
  }

  /// The last `lines` lines of the log.
  pub fn log_tail(&self, lines: usize) -> String {
    let text = fs::read(self.dir.join(LOG_FILE)).map(|bytes| String::from_utf8_lossy(&bytes).into_owned()).unwrap_or_default();
    let all: Vec<_> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
  }

  /// Writes a crash report to its own file and keeps only the newest few.
  pub fn write_crash(&self, text: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(&self.dir)?;
    let path = loop {
      let name = format!("crash-{:012}-{:04}.txt", now(), NEXT_CRASH.fetch_add(1, Ordering::Relaxed));
      let path = self.dir.join(name);
      match private_options(OpenOptions::new().write(true).create_new(true)).open(&path) {
        Ok(mut file) => {
          file.write_all(text.as_bytes())?;
          break path;
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
        Err(error) => return Err(error),
      }
    };
    let crashes = self.crashes();
    for old in &crashes[..crashes.len().saturating_sub(CRASH_LIMIT)] {
      let _ = fs::remove_file(old);
    }
    Ok(path)
  }

  /// The newest crash report, whether or not it was shown before.
  pub fn latest_crash(&self) -> Option<String> {
    let path = self.crashes().pop()?;
    fs::read(path).ok().map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
  }

  /// The newest crash report that was not announced yet. Every report found is marked as
  /// announced, so the user is told once per crash and never nagged.
  pub fn take_unannounced_crash(&self) -> Option<String> {
    let fresh: Vec<_> = self.crashes().into_iter().filter(|path| !is_seen(path)).collect();
    let text = fs::read(fresh.last()?).ok().map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
    for path in fresh {
      let seen = path.with_extension("seen.txt");
      let _ = fs::rename(path, seen);
    }
    text
  }

  /// Crash report files, oldest first.
  fn crashes(&self) -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(&self.dir)
      .into_iter()
      .flatten()
      .flatten()
      .map(|entry| entry.path())
      .filter(|path| path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.starts_with("crash-") && name.ends_with(".txt")))
      .collect();
    paths.sort();
    paths
  }
}

fn is_seen(path: &Path) -> bool {
  path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.ends_with(SEEN_SUFFIX))
}

fn now() -> u64 {
  SystemTime::now().duration_since(UNIX_EPOCH).map(|duration| duration.as_secs()).unwrap_or(0)
}

/// Logs and reports can quote commands and paths, so only the owner may read them.
fn private_options(options: &mut OpenOptions) -> &mut OpenOptions {
  #[cfg(unix)]
  {
    use std::os::unix::fs::OpenOptionsExt as _;
    options.mode(0o600);
  }
  options
}

/// Replaces the home folder in `text` with `~`, so a report does not carry the user name.
pub fn scrub_home(text: &str, home: &Path) -> String {
  let home = home.to_string_lossy();
  if home.len() <= 1 {
    return text.to_string();
  }
  text.replace(home.as_ref(), "~")
}

/// The address that opens a new GitHub issue with `title` and `body` filled in. The body is cut
/// where the address would become too long to open.
pub fn issue_url(repository: &str, title: &str, body: &str) -> String {
  let mut url = format!("{}/issues/new?title={}&body=", repository.trim_end_matches('/'), percent_encode(title));
  const ELLIPSIS: &str = "%E2%80%A6";
  for character in body.chars() {
    let piece = percent_encode(character.encode_utf8(&mut [0; 4]));
    if url.len() + piece.len() + ELLIPSIS.len() > MAX_URL_LENGTH {
      url.push_str(ELLIPSIS);
      break;
    }
    url.push_str(&piece);
  }
  url
}

fn percent_encode(text: &str) -> String {
  let mut encoded = String::with_capacity(text.len());
  for byte in text.bytes() {
    if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
      encoded.push(byte as char);
    } else {
      encoded.push_str(&format!("%{byte:02X}"));
    }
  }
  encoded
}
