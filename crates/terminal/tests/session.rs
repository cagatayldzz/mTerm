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
