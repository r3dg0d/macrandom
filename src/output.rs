//! Human and JSON output helpers.

use serde::Serialize;
use std::io::{self, Write};

#[derive(Debug, Clone, Copy)]
pub struct OutputOpts {
    pub json: bool,
    pub quiet: bool,
}

impl OutputOpts {
    pub fn print_human(&self, msg: &str) {
        if !self.json && !self.quiet {
            println!("{msg}");
        }
    }

    pub fn print_json<T: Serialize>(&self, value: &T) -> crate::error::Result<()> {
        if self.json {
            serde_json::to_writer_pretty(io::stdout(), value)?;
            println!();
        }
        Ok(())
    }

    pub fn emit<T: Serialize>(&self, human: &str, value: &T) -> crate::error::Result<()> {
        if self.json {
            self.print_json(value)?;
        } else if !self.quiet {
            println!("{human}");
        }
        Ok(())
    }

    pub fn emit_multiline<T: Serialize>(
        &self,
        lines: &[String],
        value: &T,
    ) -> crate::error::Result<()> {
        if self.json {
            self.print_json(value)?;
        } else if !self.quiet {
            for line in lines {
                println!("{line}");
            }
        }
        Ok(())
    }
}

pub fn stderr_error(err: &dyn std::fmt::Display) {
    let _ = writeln!(io::stderr(), "error: {err}");
}
