use std::{
    io::{Read, Write},
    path::Path,
    sync::mpsc::Sender,
    thread,
};

use anyhow::{Context, Result};
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

/// A program on a pseudo-terminal: it sees a real terminal, so prompts, colors and ssh's
/// questions work, while its screen is drawn inside the app.
pub struct Pty {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

fn size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

impl Pty {
    /// Everything it prints is sent as raw bytes, for a terminal parser to draw.
    pub fn spawn(
        prog: &str,
        args: &[String],
        envs: &[(String, String)],
        cwd: &Path,
        (rows, cols): (u16, u16),
        output: Sender<Vec<u8>>,
    ) -> Result<Self> {
        let pair = native_pty_system()
            .openpty(size(rows, cols))
            .context("could not open a terminal")?;
        let mut cmd = CommandBuilder::new(super::os::program(prog));
        cmd.args(args);
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");
        // COLORTERM and the rest of the environment are the terminal's own, so a program here
        // draws in the same colors it would in that terminal.
        for (key, value) in envs {
            cmd.env(key, value);
        }
        let child = pair
            .slave
            .spawn_command(cmd)
            .with_context(|| super::run::start_failure(prog))?;
        // Its last handle closed, the reader below sees the end once the program exits.
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader()?;
        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            while let Ok(n @ 1..) = reader.read(&mut buf) {
                if output.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
        let writer = pair.master.take_writer()?;
        Ok(Self {
            master: pair.master,
            writer,
            child,
        })
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn resize(&self, (rows, cols): (u16, u16)) -> Result<()> {
        self.master.resize(size(rows, cols))
    }

    /// `Some(exit code)` once it has ended.
    pub fn exited(&mut self) -> Option<u32> {
        self.child.try_wait().ok().flatten().map(|s| s.exit_code())
    }

    /// Closing the terminal hangs up everything it started, a dev server's children included.
    pub fn stop(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
