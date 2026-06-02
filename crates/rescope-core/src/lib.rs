pub mod aggregate;
pub mod error;
pub mod filter;
pub mod metrics;
pub mod report;
pub mod sampling;
pub mod sort;
pub mod units;

pub use aggregate::{
    RecordingAccumulator, RecordingAccumulatorOptions, aggregate_recording, aggregate_snapshot,
};
pub use error::RescopeError;
pub use filter::{CompiledFilter, filter_sample};
pub use metrics::*;
pub use report::{
    RecordingReportOptions, SnapshotReportOptions, build_recording_report,
    build_recording_report_from_accumulator, build_snapshot_report, platform_notes,
};
pub use sampling::{SampleSource, SamplerConfig, SysinfoSampler};
pub use units::{format_bps, format_bytes, format_signed_bytes, parse_duration, sparkline};
