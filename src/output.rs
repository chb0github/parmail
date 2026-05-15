use indicatif::{ProgressBar, ProgressStyle};

use crate::models::EmailManifest;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Silent,
    Quiet,
    Normal,
    Verbose,
    Debug,
}

impl Verbosity {
    pub fn from_flags(verbose: u8, quiet: u8) -> Self {
        match (verbose, quiet) {
            (_, 2..) => Verbosity::Silent,
            (_, 1) => Verbosity::Quiet,
            (0, 0) => Verbosity::Normal,
            (1, _) => Verbosity::Verbose,
            (2.., _) => Verbosity::Debug,
        }
    }
}

pub struct Output {
    verbosity: Verbosity,
    interactive: bool,
    progress: Option<ProgressBar>,
}

impl Output {
    pub fn new(verbosity: Verbosity, interactive: bool, total: u64) -> Self {
        let progress = if interactive && verbosity >= Verbosity::Quiet && verbosity <= Verbosity::Normal {
            let pb = ProgressBar::new(total);
            let style = ProgressStyle::with_template(
                "{spinner:.green} [{bar:30.cyan/blue}] {pos}/{len} ({percent}%) {msg}"
            )
            .unwrap()
            .progress_chars("=>-");
            pb.set_style(style);
            Some(pb)
        } else {
            None
        };

        Self {
            verbosity,
            interactive,
            progress,
        }
    }

    pub fn start_file(&self, filename: &str) {
        if let Some(pb) = &self.progress {
            pb.set_message(filename.to_string());
        }
        if self.verbosity >= Verbosity::Verbose {
            eprintln!("Processing: {filename}");
        }
    }

    pub fn step(&self, msg: &str) {
        if self.verbosity >= Verbosity::Verbose {
            eprintln!("  {msg}");
        }
    }

    pub fn file_done(&self, subject: &str, image_count: usize) {
        if let Some(pb) = &self.progress {
            pb.inc(1);
            if self.verbosity >= Verbosity::Normal {
                pb.set_message(format!("{subject} — {image_count} images"));
            }
        } else if self.verbosity >= Verbosity::Normal {
            if self.interactive {
                eprint!("\r\x1b[K  {subject} — {image_count} images");
            } else {
                eprintln!("{subject} — {image_count} images");
            }
        }
    }

    pub fn dump_json(&self, manifest: &EmailManifest) {
        if self.verbosity >= Verbosity::Debug {
            if let Ok(json) = serde_json::to_string_pretty(manifest) {
                eprintln!("{json}");
            }
        }
    }

    pub fn error(&self, msg: &str) {
        if let Some(pb) = &self.progress {
            pb.suspend(|| eprintln!("ERROR: {msg}"));
        } else {
            eprintln!("ERROR: {msg}");
        }
    }

    pub fn finish(&self, total: u64, errors: u64) {
        if let Some(pb) = &self.progress {
            pb.finish_and_clear();
        }
        if self.interactive && self.verbosity == Verbosity::Normal {
            eprintln!();
        }
        if self.verbosity >= Verbosity::Normal {
            let status = if errors == 0 {
                format!("Done. Processed {total} emails.")
            } else {
                format!("Done. Processed {total} emails ({errors} errors).")
            };
            eprintln!("{status}");
        }
    }
}
