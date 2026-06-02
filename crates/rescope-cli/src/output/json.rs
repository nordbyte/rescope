use std::fs::File;
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

fn write_report<T>(path: &Path, mode: &'static str, report: &T) -> Result<()>
where
    T: Serialize,
{
    let file = File::create(path)?;
    let envelope = ExportEnvelope {
        tool: "rescope",
        version: env!("CARGO_PKG_VERSION"),
        mode,
        report,
    };
    serde_json::to_writer_pretty(file, &envelope)?;
    Ok(())
}
