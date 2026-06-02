use anyhow::{Context, Result};
use rescope_core::{
    SampleSource, SamplerConfig, SysinfoSampler, build_snapshot_report, filter_sample,
};

use crate::args::{Cli, SnapshotArgs};
use crate::output::{csv, json, table};

pub fn run(cli: &Cli, args: &SnapshotArgs) -> Result<()> {
    let filter = args.filters.to_filter_spec();
    let mut sampler = SysinfoSampler::new(SamplerConfig {
        include_command: args.filters.needs_command(),
    })?;
    sampler.warm_up(args.interval)?;

    let sample = sampler.sample()?;
    let filtered = filter_sample(&sample, &filter);
    let report = build_snapshot_report(
        &filtered,
        args.interval,
        args.group.into(),
        args.sort.into(),
        filter,
        args.filters.show_command,
        args.limit,
    );

    if let Some(path) = &cli.json {
        json::write_snapshot(path, &report)
            .with_context(|| format!("writing {}", path.display()))?;
    }
    if let Some(path) = &cli.csv {
        csv::write_snapshot(path, &report)
            .with_context(|| format!("writing {}", path.display()))?;
    }

    if !cli.quiet {
        table::print_snapshot(&report, cli.bytes, args.show_system || !cli.quiet);
    }

    Ok(())
}
