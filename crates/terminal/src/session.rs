use crate::{
  palette::Palette,
  snapshot::{spans, Cursor, Glyph, Snapshot},
};
use alacritty_terminal::{
  event::{Event, EventListener, WindowSize},
  event_loop::{EventLoop, EventLoopSender, Msg},
  grid::{Dimensions, Scroll},
  sync::FairMutex,
  term::{cell::Flags, Config, Term, TermMode},
  tty::{self, Options, Shell},
  vte::ansi::NamedColor,
};
use std::{
  path::PathBuf,
  sync::{Arc, Mutex, OnceLock},
};

/// What a session tells the interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionEvent {
  /// The screen changed.
  Wakeup,
  /// The program set a window title.
  Title(String),
  /// The shell reported its working directory.
  Directory(PathBuf),
  /// The shell exited.
  Exit,
}

/// How to start a session.
#[derive(Clone, Debug, Default)]
pub struct SessionOptions {
  /// A program and its arguments; `None` starts the user's login shell.
  pub program: Option<(String, Vec<String>)>,
  pub working_directory: Option<PathBuf>,
  /// The initial colors used to answer terminal color queries.
  pub palette: Option<Palette>,
}

/// Terminal modes that change how input is encoded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modes {
  pub app_cursor: bool,
  pub bracketed_paste: bool,
}

type Notify = Arc<dyn Fn(SessionEvent) + Send + Sync>;

#[derive(Clone)]
struct Proxy {
  notify: Notify,
  sender: Arc<OnceLock<EventLoopSender>>,
  palette: Arc<Mutex<Palette>>,
}

impl EventListener for Proxy {
  fn send_event(&self, event: Event) {
    match event {
      Event::Wakeup => (self.notify)(SessionEvent::Wakeup),
      Event::Title(title) => (self.notify)(SessionEvent::Title(title)),
      Event::ResetTitle => (self.notify)(SessionEvent::Title(String::new())),
      Event::Exit | Event::ChildExit(_) => (self.notify)(SessionEvent::Exit),
      // Replies the program asked for, such as cursor position reports.
      Event::PtyWrite(text) => {
        if let Some(sender) = self.sender.get() {
          let _ = sender.send(Msg::Input(text.into_bytes().into()));
        }
      }
      Event::ColorRequest(index, format) => {
        let palette = self.palette.lock().unwrap_or_else(|error| error.into_inner());
        let color = match index {
          0..=255 => palette.indexed(index as u8),
          index if index == NamedColor::Background as usize => palette.background,
          _ => palette.foreground,
        };
        if let Some(sender) = self.sender.get() {
          let _ = sender.send(Msg::Input(format(alacritty_terminal::vte::ansi::Rgb { r: color.r, g: color.g, b: color.b }).into_bytes().into()));
        }
      }
      _ => {}
    }
  }
}

struct Size {
  columns: usize,
  lines: usize,
}

impl Dimensions for Size {
  fn total_lines(&self) -> usize {
    self.lines
  }

  fn screen_lines(&self) -> usize {
    self.lines
  }

  fn columns(&self) -> usize {
    self.columns
  }
}

/// A running shell attached to a pseudo-terminal.
pub struct Session {
  term: Arc<FairMutex<Term<Proxy>>>,
  sender: EventLoopSender,
  palette: Arc<Mutex<Palette>>,
  #[cfg(unix)]
  child_id: u32,
  notify: Notify,
  #[cfg(unix)]
  directory_refresh: Arc<Mutex<DirectoryRefresh>>,
}

#[cfg(unix)]
#[derive(Default)]
struct DirectoryRefresh {
  running: bool,
  pending: bool,
  closed: bool,
}

impl Session {
  /// Starts the shell. `notify` is called from a background thread whenever
  /// something happens; it must be cheap and must not block.
  pub fn spawn(
    options: SessionOptions,
    columns: usize,
    lines: usize,
    notify: impl Fn(SessionEvent) + Send + Sync + 'static,
  ) -> std::io::Result<Self> {
    tty::setup_env();
    let (columns, lines) = (columns.max(2), lines.max(1));
    let palette = Arc::new(Mutex::new(options.palette.unwrap_or_else(Palette::dark)));
    let proxy = Proxy { notify: Arc::new(notify), sender: Arc::new(OnceLock::new()), palette: palette.clone() };
    let term = Arc::new(FairMutex::new(Term::new(Config::default(), &Size { columns, lines }, proxy.clone())));
    let pty_options = Options {
      shell: options.program.map(|(program, args)| Shell::new(program, args)),
      working_directory: options.working_directory,
      // Do not advertise the terminal that launched mTerm to child programs.
      env: [("TERM_PROGRAM".into(), "mTerm".into()), ("TERM_PROGRAM_VERSION".into(), String::new())].into(),
      ..Options::default()
    };
    let pty = tty::new(&pty_options, window_size(columns, lines), 0)?;
    #[cfg(unix)]
    let child_id = pty.child().id();
    // Drain on exit: output the program wrote just before it exited still reaches the screen.
    let event_loop = EventLoop::new(term.clone(), proxy.clone(), pty, true, false)?;
    let sender = event_loop.channel();
    let _ = proxy.sender.set(sender.clone());
    event_loop.spawn();
    Ok(Self {
      term,
      sender,
      palette,
      #[cfg(unix)]
      child_id,
      notify: proxy.notify.clone(),
      #[cfg(unix)]
      directory_refresh: Arc::new(Mutex::new(DirectoryRefresh::default())),
    })
  }

  /// Sends bytes to the program, as if typed.
  pub fn write(&self, bytes: impl Into<std::borrow::Cow<'static, [u8]>>) {
    let _ = self.sender.send(Msg::Input(bytes.into()));
    // Typing returns the view to the live output.
    self.term.lock().scroll_display(Scroll::Bottom);
  }

  /// Refreshes the shell's working directory without blocking the interface.
  pub fn refresh_directory(&self) {
    #[cfg(unix)]
    {
      let mut state = self.directory_refresh.lock().unwrap_or_else(|error| error.into_inner());
      state.pending = true;
      if state.running {
        return;
      }
      state.running = true;
      drop(state);
      let id = self.child_id;
      let notify = self.notify.clone();
      let refresh = self.directory_refresh.clone();
      std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(300));
        {
          let mut state = refresh.lock().unwrap_or_else(|error| error.into_inner());
          if state.closed {
            return;
          }
          state.pending = false;
        }
        #[cfg(target_os = "macos")]
        let directory = std::process::Command::new("/usr/sbin/lsof")
          .args(["-a", "-p", &id.to_string(), "-d", "cwd", "-Fn"])
          .output()
          .ok()
          .and_then(|output| String::from_utf8(output.stdout).ok())
          .and_then(|text| text.lines().find_map(|line| line.strip_prefix('n').map(PathBuf::from)));
        #[cfg(not(target_os = "macos"))]
        let directory = std::fs::read_link(format!("/proc/{id}/cwd")).ok();
        let mut state = refresh.lock().unwrap_or_else(|error| error.into_inner());
        if state.closed {
          return;
        }
        if !state.pending {
          state.running = false;
          drop(state);
          if let Some(directory) = directory.filter(|path| path.is_dir()) {
            notify(SessionEvent::Directory(directory));
          }
          return;
        }
      });
    }
  }

  /// Resizes the terminal and the PTY. Returns whether the size changed.
  pub fn resize(&self, columns: usize, lines: usize) -> bool {
    let (columns, lines) = (columns.max(2), lines.max(1));
    let mut term = self.term.lock();
    if term.columns() == columns && term.screen_lines() == lines {
      return false;
    }
    term.resize(Size { columns, lines });
    drop(term);
    let _ = self.sender.send(Msg::Resize(window_size(columns, lines)));
    true
  }

  /// Scrolls the history view; positive lines go back in time.
  pub fn scroll(&self, lines: i32) {
    self.term.lock().scroll_display(Scroll::Delta(lines));
  }

  pub fn modes(&self) -> Modes {
    let term = self.term.lock();
    Modes { app_cursor: term.mode().contains(TermMode::APP_CURSOR), bracketed_paste: term.mode().contains(TermMode::BRACKETED_PASTE) }
  }

  /// Updates the colors reported to applications querying the terminal.
  pub fn set_palette(&self, palette: Palette) {
    *self.palette.lock().unwrap_or_else(|error| error.into_inner()) = palette;
  }

  /// The current screen, colored with `palette`.
  pub fn snapshot(&self, palette: &Palette) -> Snapshot {
    let term = self.term.lock();
    let (columns, lines) = (term.columns(), term.screen_lines());
    let content = term.renderable_content();
    let offset = content.display_offset;
    let mut rows = vec![vec![Glyph::blank(); columns]; lines];
    let mut hyperlinks = vec![vec![None; columns]; lines];
    for row in &mut rows {
      for glyph in row.iter_mut() {
        glyph.foreground = palette.foreground;
      }
    }
    for indexed in content.display_iter {
      let line = indexed.point.line.0 + offset as i32;
      let column = indexed.point.column.0;
      if line < 0 || line as usize >= lines || column >= columns {
        continue;
      }
      let cell = indexed.cell;
      hyperlinks[line as usize][column] = cell.hyperlink().map(|link| link.uri().to_owned());
      if cell.flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
        // The wide character before it already fills both cells.
        rows[line as usize][column] = Glyph { character: '\u{200b}', ..Glyph::blank() };
        continue;
      }
      let background = palette.resolve(cell.bg);
      let mut glyph = Glyph {
        character: if cell.flags.contains(Flags::HIDDEN) { ' ' } else { cell.c },
        foreground: palette.resolve(cell.fg),
        background: (background != palette.background).then_some(background),
        bold: cell.flags.contains(Flags::BOLD),
        italic: cell.flags.contains(Flags::ITALIC),
        underline: cell.flags.intersects(Flags::ALL_UNDERLINES),
        strikeout: cell.flags.contains(Flags::STRIKEOUT),
      };
      if cell.flags.contains(Flags::INVERSE) {
        glyph.invert(palette);
      }
      if cell.flags.contains(Flags::DIM) {
        glyph.foreground = glyph.foreground.faint_on(glyph.background.unwrap_or(palette.background));
      }
      rows[line as usize][column] = glyph;
    }
    let cursor = alacritty_terminal::term::point_to_viewport(offset, content.cursor.point)
      .filter(|_| content.cursor.shape != alacritty_terminal::vte::ansi::CursorShape::Hidden)
      .filter(|point| point.line < lines && point.column.0 < columns)
      .map(|point| Cursor { line: point.line, column: point.column.0 });
    if let Some(cursor) = cursor {
      rows[cursor.line][cursor.column].invert(palette);
    }
    let lines = rows.iter().map(spans).collect();
    Snapshot { lines, cursor, columns, scrolled: offset, hyperlinks }
  }
}

impl Drop for Session {
  fn drop(&mut self) {
    #[cfg(unix)]
    {
      self.directory_refresh.lock().unwrap_or_else(|error| error.into_inner()).closed = true;
    }
    let _ = self.sender.send(Msg::Shutdown);
  }
}

fn window_size(columns: usize, lines: usize) -> WindowSize {
  WindowSize { num_lines: lines as u16, num_cols: columns as u16, cell_width: 1, cell_height: 1 }
}
