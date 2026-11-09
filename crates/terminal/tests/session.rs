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
fn scrolling_tracks_history_and_reports_viewport_changes() {
  let (session, receiver) = run("i=0; while [ $i -lt 30 ]; do echo line$i; i=$((i + 1)); done");
  wait_for_exit(&receiver);
  wait_for_screen(&session, |snapshot| snapshot.history_size > 0);
  let history = session.snapshot(&Palette::light()).history_size;
  assert!(session.scroll_to(usize::MAX));
  assert_eq!(session.snapshot(&Palette::light()).scrolled, history);
  assert!(!session.scroll(1));
  assert!(session.scroll_to(0));
  assert!(!session.scroll_to(0));
  assert_eq!(session.snapshot(&Palette::light()).scrolled, 0);
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

#[cfg(unix)]
#[test]
fn reads_the_shell_working_directory_on_demand() {
  let (session, _receiver) = run("cd /tmp; printf ready; sleep 3");
  wait_for_screen(&session, |snapshot| snapshot.text().contains("ready"));
  let directory = session.current_directory().expect("the shell directory");
  assert_eq!(directory.canonicalize().unwrap(), std::path::Path::new("/tmp").canonicalize().unwrap());
}

#[test]
fn programs_are_told_the_terminal_type_and_a_locale() {
  let (session, receiver) = run("printf '%s|%s' \"$TERM\" \"${LANG:-none}\"");
  wait_for_exit(&receiver);
  wait_for_screen(&session, |snapshot| snapshot.text().contains('|'));
  let text = session.snapshot(&Palette::light()).text();
  assert!(text.starts_with("xterm-256color|"), "{text}");
  assert!(!text.ends_with("|none"), "{text}");
}

#[test]
fn mouse_and_screen_modes_follow_the_program() {
  let (session, _receiver) = run("printf '\\033[?1002h\\033[?1006h\\033[?1049h'; sleep 2");
  let deadline = Instant::now() + Duration::from_secs(5);
  while Instant::now() < deadline && !session.modes().alt_screen {
    std::thread::sleep(Duration::from_millis(20));
  }
  let modes = session.modes();
  assert!(modes.mouse_drag && modes.sgr_mouse && modes.alt_screen && modes.mouse_reporting());
  assert!(!modes.mouse_motion && !modes.focus_reporting);
}

#[test]
fn clearing_the_history_keeps_the_screen() {
  let (session, receiver) = run("i=0; while [ $i -lt 30 ]; do echo line$i; i=$((i + 1)); done");
  wait_for_exit(&receiver);
  wait_for_screen(&session, |snapshot| snapshot.history_size > 0);
  session.clear_history();
  let snapshot = session.snapshot(&Palette::light());
  assert_eq!((snapshot.history_size, snapshot.scrolled), (0, 0));
  assert!(snapshot.text().contains("line29"));
}

#[test]
fn text_that_wraps_is_marked_and_separate_lines_are_not() {
  let (session, receiver) = run("printf '%050d\\nshort' 7");
  wait_for_exit(&receiver);
  wait_for_screen(&session, |snapshot| snapshot.text().contains("short"));
  let snapshot = session.snapshot(&Palette::light());
  assert_eq!(snapshot.wrapped[..3], [true, false, false]);
}

#[test]
fn hyperlinks_report_the_cells_they_cover() {
  let (session, receiver) = run("printf 'go \\033]8;;https://example.org/\\007here\\033]8;;\\007 now'");
  wait_for_exit(&receiver);
  let snapshot = session.snapshot(&Palette::light());
  assert!(snapshot.text().contains("now"));
  assert_eq!(snapshot.link_range_at(0, 5), Some(("https://example.org/".to_owned(), 3, 7)));
  assert_eq!(snapshot.link_range_at(0, 8), None);
}

#[test]
fn clipboard_requests_from_programs_reach_the_interface() {
  let (_session, receiver) = run("printf '\\033]52;c;aGVsbG8=\\007'; sleep 1");
  let deadline = Instant::now() + Duration::from_secs(5);
  while Instant::now() < deadline {
    if let Ok(SessionEvent::ClipboardStore(text)) = receiver.recv_timeout(Duration::from_millis(100)) {
      assert_eq!(text, "hello");
      return;
    }
  }
  panic!("no clipboard request arrived");
}

#[test]
fn search_covers_history_and_ignores_case_unless_asked() {
  let (session, receiver) = run("i=0; while [ $i -lt 30 ]; do echo Needle$i; i=$((i + 1)); done");
  wait_for_exit(&receiver);
  wait_for_screen(&session, |snapshot| snapshot.history_size > 0 && snapshot.text().contains("Needle29"));
  let hits = session.find("needle1", false);
  assert_eq!(hits.len(), 11);
  assert!(hits[0].line < 0, "the oldest hit lives in the history");
  assert_eq!((hits[0].start, hits[0].end), (0, 7));
  assert_eq!(session.find("needle1", true).len(), 0);
  assert_eq!(session.find("", false).len(), 0);
}

#[test]
fn revealing_a_hit_scrolls_to_it_and_highlights_mark_the_current_one() {
  let (session, receiver) = run("i=0; while [ $i -lt 30 ]; do echo Needle$i; i=$((i + 1)); done");
  wait_for_exit(&receiver);
  wait_for_screen(&session, |snapshot| snapshot.history_size > 0 && snapshot.text().contains("Needle29"));
  let hits = session.find("needle", false);
  let oldest = hits[0];
  assert!(session.highlights("needle", false, None).iter().all(|highlight| !highlight.current));
  assert!(session.reveal(oldest));
  assert!(!session.reveal(oldest));
  let current: Vec<_> = session.highlights("needle", false, Some(oldest)).into_iter().filter(|highlight| highlight.current).collect();
  assert_eq!(current.len(), 1);
  assert_eq!((current[0].start, current[0].end), (0, 6));
}

#[test]
fn search_measures_wide_characters_in_cells() {
  let (session, receiver) = run("printf 'a界b 界b'");
  wait_for_exit(&receiver);
  wait_for_screen(&session, |snapshot| snapshot.text().contains("界b"));
  let hits = session.find("界b", true);
  assert_eq!(hits.iter().map(|hit| (hit.start, hit.end)).collect::<Vec<_>>(), [(1, 4), (5, 8)]);
}
