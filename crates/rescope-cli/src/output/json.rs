use std::io;
use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use rescope_core::{RecordingReport, SnapshotReport};

#[derive(Debug, Serialize)]
struct ExportEnvelope<'a, T> {
    tool: &'static str,
    version: &'static str,
    mode: &'static str,
    #[serde(flatten)]
    report: &'a T,
}

pub fn write_snapshot(path: &Path, report: &SnapshotReport) -> Result<()> {
    write_report(path, "snapshot", report)
}

pub fn write_recording(path: &Path, report: &RecordingReport) -> Result<()> {
    write_report(path, "record", report)
}

pub fn write_custom<T>(path: &Path, mode: &'static str, report: &T) -> Result<()>
where
    T: Serialize,
{
    write_report(path, mode, report)
}

pub fn writes_stdout(path: &Option<std::path::PathBuf>) -> bool {
    path.as_deref() == Some(Path::new("-"))
}

fn write_report<T>(path: &Path, mode: &'static str, report: &T) -> Result<()>
where
    T: Serialize,
{
    let envelope = ExportEnvelope {
        tool: "rescope",
        version: env!("CARGO_PKG_VERSION"),
        mode,
        report,
    };
    if path == Path::new("-") {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        serde_json::to_writer_pretty(&mut handle, &envelope)?;
        use std::io::Write;
        writeln!(handle)?;
        return Ok(());
    }

    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let mut temp_file = match parent {
        Some(parent) => tempfile::NamedTempFile::new_in(parent)?,
        None => tempfile::NamedTempFile::new_in(".")?,
    };
    serde_json::to_writer_pretty(&mut temp_file, &envelope)?;
    temp_file.as_file_mut().sync_all()?;
    temp_file.persist(path)?;
    Ok(())
}
