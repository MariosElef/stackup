use std::sync::mpsc::{Receiver, channel};

use anyhow::Result;
use tui_term::vt100;

use crate::{clients::pty::Pty, models::Step};

/// An action running on the in-app terminal: its steps one after another, each on a fresh
/// terminal, all drawn into one screen.
pub struct Run {
    pub title: String,
    steps: Vec<Step>,
    next: usize,
    pty: Option<Pty>,
    rx: Option<Receiver<Vec<u8>>>,
    /// The step's exit code, seen once; the output is read one more time before it is acted on.
    exited: Option<u32>,
    pub term: vt100::Parser,
    size: (u16, u16),
    /// How it ended: "done", "failed (exit 1)", "stopped". `None` while something is running.
    pub ended: Option<String>,
    /// Set once every step succeeded.
    pub succeeded: bool,
    /// What the programs printed, kept (the last of it) so a failure can be diagnosed.
    log: Vec<u8>,
}

/// How much output is kept for diagnosing.
const LOG_LIMIT: usize = 64 * 1024;

impl Run {
    pub fn new(title: &str, steps: Vec<Step>, size: (u16, u16)) -> Self {
        Self {
            title: title.to_string(),
            steps,
            next: 0,
            pty: None,
            rx: None,
            exited: None,
            term: vt100::Parser::new(size.0, size.1, 0),
            size,
            ended: None,
            succeeded: false,
            log: Vec::new(),
        }
    }

    /// The steps this was started with, so they can be run again after a fix.
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// Everything the programs printed, without the terminal's escape codes.
    pub fn log_text(&self) -> String {
        strip_escapes(&String::from_utf8_lossy(&self.log))
    }

    pub fn live(&self) -> bool {
        self.ended.is_none()
    }

    /// Which step is running, for the title: "2/7".
    pub fn progress(&self) -> String {
        format!("{}/{}", self.next.min(self.steps.len()), self.steps.len())
    }

    fn say(&mut self, text: &str) {
        self.term.process(text.as_bytes());
    }

    /// Starts the next step on a new terminal, after a line saying what it is.
    pub fn start_next(&mut self) -> Result<()> {
        let step = self.steps[self.next].clone();
        self.next += 1;
        self.say(&format!(
            "\x1b[38;5;45m# {}\x1b[0m\r\n\x1b[2m$ {}\x1b[0m\r\n",
            step.label,
            step.command_line()
        ));
        let (tx, rx) = channel();
        self.pty = Some(Pty::spawn(
            &step.prog, &step.args, &step.env, &step.cwd, self.size, tx,
        )?);
        self.rx = Some(rx);
        self.exited = None;
        Ok(())
    }

    /// Draws `bytes` on the screen, and answers the questions a program asks its terminal. A real
    /// terminal replies to "where is the cursor?" (`ESC [ 6 n`) and "what are you?" (`ESC [ c`);
    /// without a reply, programs that ask first (gh's prompts, for one) wait for it forever and
    /// take the keys typed meanwhile for the answer.
    fn process_answering(&mut self, bytes: &[u8]) {
        let mut rest = bytes;
        loop {
            let found = [
                (&b"\x1b[6n"[..], true),
                (&b"\x1b[c"[..], false),
                (&b"\x1b[0c"[..], false),
            ]
            .iter()
            .filter_map(|(q, cursor)| {
                rest.windows(q.len())
                    .position(|w| w == *q)
                    .map(|at| (at, q.len(), *cursor))
            })
            .min();
            let Some((at, len, cursor)) = found else {
                break;
            };
            // What was printed before the question is on the screen when it is answered.
            self.term.process(&rest[..at]);
            let reply = if cursor {
                let (row, col) = self.term.screen().cursor_position();
                format!("\x1b[{};{}R", row + 1, col + 1)
            } else {
                // A VT100 with the advanced video option: the plainest thing that answers.
                "\x1b[?1;2c".to_string()
            };
            if let Some(pty) = &mut self.pty {
                let _ = pty.write(reply.as_bytes());
            }
            rest = &rest[at + len..];
        }
        self.term.process(rest);
    }

    fn drain(&mut self) {
        let Some(rx) = &self.rx else { return };
        let mut got = Vec::new();
        while let Ok(bytes) = rx.try_recv() {
            got.extend(bytes);
        }
        self.process_answering(&got);
        self.log.extend_from_slice(&got);
        if self.log.len() > LOG_LIMIT {
            self.log.drain(..self.log.len() - LOG_LIMIT);
        }
    }

    /// Moves it along: draws what the program printed, notices when it ends, and starts the
    /// next step. Called often, with the size the pane has now.
    pub fn pump(&mut self, size: (u16, u16)) {
        if size != self.size && size.0 > 0 && size.1 > 0 {
            self.size = size;
            self.term.screen_mut().set_size(size.0, size.1);
            if let Some(pty) = &self.pty {
                let _ = pty.resize(size);
            }
        }
        self.drain();
        let Some(pty) = &mut self.pty else { return };
        let Some(code) = pty.exited() else { return };
        // The reader thread may still be handing over the last of its output: look once more
        // on the next call before acting on the exit.
        if self.exited.is_none() {
            self.exited = Some(code);
            return;
        }
        self.pty = None;
        self.rx = None;
        if code == 0 && self.next < self.steps.len() {
            if let Err(e) = self.start_next() {
                self.say(&format!("\r\n{e:#}\r\n"));
                self.ended = Some("failed".into());
            }
        } else if code == 0 {
            self.succeeded = true;
            self.ended = Some("done".into());
        } else {
            self.ended = Some(format!("failed (exit {code})"));
        }
    }

    pub fn type_bytes(&mut self, bytes: &[u8]) {
        if let Some(pty) = &mut self.pty {
            let _ = pty.write(bytes);
        }
    }

    /// Stops what is running; closing the terminal hangs up everything it started.
    pub fn stop(&mut self) {
        if let Some(pty) = self.pty.take() {
            pty.stop();
        }
        self.rx = None;
        if self.ended.is_none() {
            self.ended = Some("stopped".into());
        }
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Text without the color and cursor codes a terminal program prints around it.
fn strip_escapes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        // `ESC [ ... final-byte` is a control sequence; anything else after ESC is one character.
        if chars.next_if_eq(&'[').is_some() {
            while chars.next().is_some_and(|c| !('@'..='~').contains(&c)) {}
        } else {
            chars.next();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_and_cursor_codes_are_stripped_from_the_log() {
        let text = "\x1b[1m\x1b[91merror\x1b[0m: it requires \x1b[2Krustc 1.99\r\n";
        assert_eq!(strip_escapes(text), "error: it requires rustc 1.99\r\n");
    }

    /// Runs `script` under bash on the in-app terminal and waits for it to end.
    #[cfg(unix)]
    fn run_script(script: &str) -> Run {
        let step = Step::new("ask", std::path::Path::new("."), "bash", &["-c", script]);
        let mut run = Run::new("ask", vec![step], (24, 80));
        run.start_next().unwrap();
        for _ in 0..300 {
            run.pump((24, 80));
            if run.ended.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        run
    }

    #[cfg(unix)]
    #[test]
    fn a_program_that_asks_where_the_cursor_is_gets_an_answer() {
        // What gh's prompts do before they read a key: ask, and wait for the reply.
        let run = run_script(
            "stty -icanon -echo; printf '\\033[6n'; IFS= read -r -t 3 -d R reply || exit 3; printf 'reply=%s\\n' \"$reply\" | cat -v",
        );
        assert_eq!(run.ended.as_deref(), Some("done"), "{}", run.log_text());
        let text = run.log_text();
        assert!(text.contains("reply=^[["), "{text}");
        // Row and column, counted from 1: the header lines are above it.
        let reply = text.split("reply=^[[").nth(1).unwrap();
        let (row, col) = reply.split_once(';').unwrap();
        assert!(row.trim().parse::<u16>().unwrap() >= 1, "{reply}");
        assert!(
            col.trim().chars().next().unwrap().is_ascii_digit(),
            "{reply}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_device_query_is_answered_too_and_normal_output_is_untouched() {
        let run = run_script(
            "stty -icanon -echo; printf 'before\\033[cafter\\n'; IFS= read -r -t 3 -d c reply || exit 3; printf 'reply=%s\\n' \"$reply\" | cat -v",
        );
        assert_eq!(run.ended.as_deref(), Some("done"), "{}", run.log_text());
        let screen = run.term.screen().contents();
        assert!(screen.contains("beforeafter"), "{screen}");
        assert!(
            run.log_text().contains("reply=^[[?1;2"),
            "{}",
            run.log_text()
        );
    }
}
