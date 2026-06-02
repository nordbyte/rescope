use std::thread;

use anyhow::Result;
use rescope_core::{
    SampleSource, SamplerConfig, SysinfoSampler, build_snapshot_report, filter_sample,
};

use crate::args::{Cli, LiveArgs};
use crate::output::{table, terminal};
use crate::tui;

pub fn run(cli: &Cli, args: &LiveArgs) -> Result<()> {
    if args.tui && !tui::is_available() && !cli.quiet {
        eprintln!("interactive TUI mode is planned; using plain terminal refresh for now");
    }

    let filter = args.filters.to_filter_spec();
    let mut sampler = SysinfoSampler::new(SamplerConfig {
        include_command: args.filters.needs_command(),
    })?;
    sampler.warm_up(args.interval)?;

    loop {
        let sample = sampler.sample()?;
        let filtered = filter_sample(&sample, &filter);
        let report = build_snapshot_report(
            &filtered,
            args.interval,
            args.group.into(),
            args.sort.into(),
            filter.clone(),
            args.filters.show_command,
            args.limit,
        );

        if !cli.quiet {
            if !args.once {
                terminal::clear_screen()?;
            }
            table::print_snapshot(&report, cli.bytes, true);
        }

        if args.once {
            break;
        }

        thread::sleep(args.interval);
    }

    Ok(())
}
