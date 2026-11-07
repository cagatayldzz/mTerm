use mterm_terminal::{Palette, Session, SessionEvent, SessionOptions};
use std::{
  sync::mpsc,
  time::{Duration, Instant},
};

fn run(script: &str) -> (Session, mpsc::Receiver<SessionEvent>) {
  let (sender, receiver) = mpsc::channel();
  let options = SessionOptions { program: Some(("/bin/sh".into(), vec!["-c".into(), script.into()])), working_directory: None, palette: None };
  let session = Session::spawn(options, 40, 5, move |event| {
    let _ = sender.send(event);
  })
  .unwrap();
  (session, receiver)
}

fn wait_for_exit(receiver: &mpsc::Receiver<SessionEvent>) {
  let deadline = Instant::now() + Duration::from_secs(10);
  while Instant::now() < deadline {
    if let Ok(SessionEvent::Exit) = receiver.recv_timeout(Duration::from_millis(100)) {
      return;
    }
  }
  panic!("the shell did not exit");
}

/// Waits until the screen satisfies `ready`; the output and the exit notice race each other.
fn wait_for_screen(session: &Session, ready: impl Fn(&mterm_terminal::Snapshot) -> bool) {
  let deadline = Instant::now() + Duration::from_secs(5);
  while Instant::now() < deadline {
    if ready(&session.snapshot(&Palette::light())) {
      return;
    }
    std::thread::sleep(Duration::from_millis(20));
  }
}

#[test]
fn program_output_reaches_the_snapshot() {
  let (session, receiver) = run("printf 'hello\\n\\033[31mred\\033[0m'");
  wait_for_exit(&receiver);
  wait_for_screen(&session, |snapshot| snapshot.line_text(1).starts_with("red"));
  let snapshot = session.snapshot(&Palette::light());
  assert_eq!(snapshot.line_text(0), "hello");
  assert!(snapshot.line_text(1).starts_with("red"));
  let red = snapshot.lines[1].iter().find(|span| span.text.starts_with("red")).unwrap();
  assert_ne!(red.foreground, Palette::light().foreground);
}

#[test]
fn faint_text_blends_with_its_cell_background() {
  let (session, receiver) = run("printf '\\033[2m59\\033[22m 60 \\033[41;2m61\\033[0m'");
  wait_for_exit(&receiver);
  wait_for_screen(&session, |snapshot| snapshot.text().contains("61"));
  for palette in [Palette::light(), Palette::dark()] {
    let snapshot = session.snapshot(&palette);
    let row = &snapshot.lines[0];
    let faint = row.iter().find(|span| span.text.contains("59")).unwrap();
    let normal = row.iter().find(|span| span.text.contains("60")).unwrap();
    let colored = row.iter().find(|span| span.text.contains("61")).unwrap();
    let blend = |a: u8, b: u8| ((a as u16 + b as u16) / 2) as u8;
    assert_eq!(faint.foreground.r, blend(palette.foreground.r, palette.background.r));
    assert_eq!(normal.foreground, palette.foreground);
    assert_eq!(colored.foreground.r, blend(palette.foreground.r, palette.ansi[1].r));
  }
}

#[test]
fn ansi_colors_follow_the_theme_and_true_colors_remain_unchanged() {
  let (session, receiver) = run("printf '\\033[31mred\\033[0m \\033[38;2;12;34;56;48;2;23;45;67mcustom\\033[0m'");
  wait_for_exit(&receiver);
  wait_for_screen(&session, |snapshot| snapshot.text().contains("custom"));
  for palette in [Palette::light(), Palette::dark()] {
    let snapshot = session.snapshot(&palette);
    let red = snapshot.lines[0].iter().find(|span| span.text.contains("red")).unwrap();
    let custom = snapshot.lines[0].iter().find(|span| span.text.contains("custom")).unwrap();
    assert_eq!(red.foreground, palette.ansi[1]);
    assert_eq!(custom.foreground, mterm_terminal::Rgb::new(12, 34, 56));
    assert_eq!(custom.background, Some(mterm_terminal::Rgb::new(23, 45, 67)));
  }
}

#[test]
fn child_program_identifies_mterm_as_its_terminal() {
  let (session, receiver) = run("printf '%s:%s' \"$TERM_PROGRAM\" \"$TERM_PROGRAM_VERSION\"");
  wait_for_exit(&receiver);
  wait_for_screen(&session, |snapshot| snapshot.text().contains("mTerm:"));
  assert!(session.snapshot(&Palette::light()).text().contains("mTerm:"));
}

#[test]
fn links_are_resolved_from_terminal_cells() {
  let (session, receiver) =
    run("printf '界 https://example.com/path).\\n\\033]8;;https://example.org/target\\007label\\033]8;;\\007\\n(/tmp/example)'");
  wait_for_exit(&receiver);
  wait_for_screen(&session, |snapshot| snapshot.text().contains("label"));
  let snapshot = session.snapshot(&Palette::light());
  assert_eq!(snapshot.link_at(0, 3).as_deref(), Some("https://example.com/path"));
  assert_eq!(snapshot.link_at(0, 0), None);
  assert_eq!(snapshot.link_at(1, 1).as_deref(), Some("https://example.org/target"));
  assert_eq!(snapshot.link_at(2, 2).as_deref(), Some("/tmp/example"));
}

#[test]
fn input_is_echoed_by_the_program() {
  let (session, receiver) = run("read line; printf \"got:%s\" \"$line\"");
  session.write(b"abc\r".to_vec());
  wait_for_exit(&receiver);
  wait_for_screen(&session, |snapshot| snapshot.text().contains("got:abc"));
  assert!(session.snapshot(&Palette::light()).text().contains("got:abc"));
}

#[test]
fn resize_changes_the_snapshot_size() {
  let (session, receiver) = run("sleep 0.2");
  session.resize(60, 8);
  let snapshot = session.snapshot(&Palette::dark());
  assert_eq!((snapshot.columns, snapshot.lines.len()), (60, 8));
  wait_for_exit(&receiver);
}

#[test]
fn background_color_queries_use_the_session_palette() {
  for palette in [Palette::light(), Palette::dark()] {
    let expected = format!(
      "{:02x}{:02x}/{:02x}{:02x}/{:02x}{:02x}",
      palette.background.r, palette.background.r, palette.background.g, palette.background.g, palette.background.b, palette.background.b
    );
    let script = format!(
      "import os,select,termios,tty; tty.setraw(0); os.write(1, b'\\x1b]11;?\\x07'); r,_,_=select.select([0],[],[],3); data=os.read(0,128) if r else b''; os.write(1, b'MATCH' if b'{expected}' in data else b'MISMATCH')"
    );
    let (sender, receiver) = mpsc::channel();
    let options =
      SessionOptions { program: Some(("python3".into(), vec!["-c".into(), script])), working_directory: None, palette: Some(palette.clone()) };
    let session = Session::spawn(options, 40, 5, move |event| {
      let _ = sender.send(event);
    })
    .unwrap();
    wait_for_exit(&receiver);
    wait_for_screen(&session, |snapshot| snapshot.text().contains("MATCH"));
    assert!(session.snapshot(&palette).text().contains("MATCH"));
    assert!(!session.snapshot(&palette).text().contains("MISMATCH"));
  }
}

#[cfg(unix)]
#[test]
fn reports_the_shell_working_directory() {
  let (session, receiver) = run("cd /tmp; printf ready; sleep 3");
  wait_for_screen(&session, |snapshot| snapshot.text().contains("ready"));
  for _ in 0..50 {
    session.refresh_directory();
  }
  let deadline = Instant::now() + Duration::from_secs(5);
  let mut directory = None;
  while Instant::now() < deadline {
    if let Ok(SessionEvent::Directory(path)) = receiver.recv_timeout(Duration::from_millis(100)) {
      directory = Some(path);
      break;
    }
  }
  assert_eq!(directory.unwrap().canonicalize().unwrap(), std::path::Path::new("/tmp").canonicalize().unwrap());
  let deadline = Instant::now() + Duration::from_millis(700);
  while Instant::now() < deadline {
    assert!(!matches!(receiver.recv_timeout(Duration::from_millis(50)), Ok(SessionEvent::Directory(_))));
  }
}
