use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail};
use rescope_core::{
    ProcessDetails, RawProcessSample, SampleSource, SamplerConfig, SortBy, SysinfoSampler,
    filter_sample, format_bytes, system_time_ms, units::MINIMUM_INTERVAL,
};
use serde::Serialize;

use crate::args::{Cli, TreeArgs};
use crate::commands::verbose;
use crate::output::{csv as output_csv, json};

#[derive(Debug, Serialize)]
struct TreeReport {
    timestamp: u64,
    interval_ms: u64,
    process_total: usize,
    matched_processes: usize,
    logical_cpu_count: usize,
    cpu_normalized: bool,
    nodes: Vec<TreeNode>,
}

#[derive(Debug, Clone, Serialize)]
struct TreeNode {
    pid: u32,
    parent_pid: Option<u32>,
    name: String,
    display_name: String,
    user_name: String,
    executable_path: Option<String>,
    command: Option<String>,
    cpu_percent: f32,
    subtree_cpu_percent: f32,
    ram_bytes: u64,
    subtree_ram_bytes: u64,
    disk_io_delta_bytes: u64,
    subtree_disk_io_delta_bytes: u64,
    details: ProcessDetails,
    children: Vec<TreeNode>,
}

pub fn run(cli: &Cli, args: &TreeArgs) -> Result<()> {
    if cli.stdout_export_count() > 1 {
        bail!("only one of --json - or --csv - can write to stdout");
    }
    rescope_core::error::validate_interval(args.interval, MINIMUM_INTERVAL)?;

    let filter = args.filters.to_filter_spec();
    let mut sampler = SysinfoSampler::new(SamplerConfig {
        include_command: args.needs_command(),
        include_executable: args.needs_executable(),
    })?;
    verbose(
        cli,
        format!(
            "tree sort={:?} limit={} command={} executable={}",
            args.effective_sort(),
            args.effective_limit(),
            args.needs_command(),
            args.needs_executable()
        ),
    );
    sampler.warm_up(args.interval)?;

    let sample = sampler.sample()?;
    let filtered = filter_sample(&sample, &filter);
    let nodes = build_tree(
        &filtered.processes,
        args.effective_sort(),
        args.effective_show_command(),
        args.effective_show_path(),
    );
    let report = TreeReport {
        timestamp: system_time_ms(filtered.timestamp),
        interval_ms: filtered.sample_interval.as_millis().min(u64::MAX as u128) as u64,
        process_total: sample.processes.len(),
        matched_processes: filtered.processes.len(),
        logical_cpu_count: filtered.logical_cpu_count,
        cpu_normalized: args.normalize_cpu,
        nodes,
    };

    if let Some(path) = &cli.json {
        json::write_custom(path, "tree", &report)
            .with_context(|| format!("writing {}", path.display()))?;
    }
    if let Some(path) = &cli.csv {
        write_tree_csv(path, &report).with_context(|| format!("writing {}", path.display()))?;
    }

    if !cli.quiet && !json::writes_stdout(&cli.json) && !output_csv::writes_stdout(&cli.csv) {
        print!(
            "{}",
            render_tree(
                &report,
                args.effective_limit(),
                cli.bytes,
                args.normalize_cpu
            )
        );
    }

    Ok(())
}

fn build_tree(
    processes: &[RawProcessSample],
    sort_by: SortBy,
    show_command: bool,
    show_path: bool,
) -> Vec<TreeNode> {
    let present_pids = processes
        .iter()
        .map(|process| process.identity.pid)
        .collect::<HashSet<_>>();
    let mut by_parent: HashMap<Option<u32>, Vec<&RawProcessSample>> = HashMap::new();
    for process in processes {
        let parent = process
            .parent_pid
            .filter(|parent_pid| present_pids.contains(parent_pid));
        by_parent.entry(parent).or_default().push(process);
    }

    build_children(None, &mut by_parent, sort_by, show_command, show_path)
}

fn build_children(
    parent_pid: Option<u32>,
    by_parent: &mut HashMap<Option<u32>, Vec<&RawProcessSample>>,
    sort_by: SortBy,
    show_command: bool,
    show_path: bool,
) -> Vec<TreeNode> {
    let Some(mut processes) = by_parent.remove(&parent_pid) else {
        return Vec::new();
    };
    processes
        .sort_by(|left, right| node_metric(right, sort_by).total_cmp(&node_metric(left, sort_by)));

    processes
        .into_iter()
        .map(|process| {
            let mut children = build_children(
                Some(process.identity.pid),
                by_parent,
                sort_by,
                show_command,
                show_path,
            );
            let subtree_cpu_percent = process.cpu_percent
                + children
                    .iter()
                    .map(|child| child.subtree_cpu_percent)
                    .sum::<f32>();
            let subtree_ram_bytes = process.memory_bytes
                + children
                    .iter()
                    .map(|child| child.subtree_ram_bytes)
                    .sum::<u64>();
            let disk_io_delta_bytes =
                process.disk_read_delta_bytes + process.disk_write_delta_bytes;
            let subtree_disk_io_delta_bytes = disk_io_delta_bytes
                + children
                    .iter()
                    .map(|child| child.subtree_disk_io_delta_bytes)
                    .sum::<u64>();
            children.sort_by(|left, right| {
                node_tree_metric(right, sort_by).total_cmp(&node_tree_metric(left, sort_by))
            });
            TreeNode {
                pid: process.identity.pid,
                parent_pid: process.parent_pid,
                name: process.identity.name.clone(),
                display_name: process.display_process(show_command, show_path),
                user_name: process.user_display(),
                executable_path: process.executable.clone(),
                command: process.command.clone(),
                cpu_percent: process.cpu_percent,
                subtree_cpu_percent,
                ram_bytes: process.memory_bytes,
                subtree_ram_bytes,
                disk_io_delta_bytes,
                subtree_disk_io_delta_bytes,
                details: process.details.clone(),
                children,
            }
        })
        .collect()
}

fn node_metric(process: &RawProcessSample, sort_by: SortBy) -> f64 {
    match sort_by {
        SortBy::Cpu | SortBy::CpuMax | SortBy::CpuP95 => process.cpu_percent as f64,
        SortBy::Ram | SortBy::RamAvg | SortBy::RamEnd => process.memory_bytes as f64,
        SortBy::Read => process.disk_read_delta_bytes as f64,
        SortBy::Write => process.disk_write_delta_bytes as f64,
        SortBy::Io | SortBy::IoAvg => {
            (process.disk_read_delta_bytes + process.disk_write_delta_bytes) as f64
        }
        SortBy::Pid => u32::MAX.saturating_sub(process.identity.pid) as f64,
        SortBy::Name | SortBy::User | SortBy::Started | SortBy::Exited => {
            process.cpu_percent as f64
        }
    }
}

fn node_tree_metric(node: &TreeNode, sort_by: SortBy) -> f64 {
    match sort_by {
        SortBy::Cpu | SortBy::CpuMax | SortBy::CpuP95 => node.subtree_cpu_percent as f64,
        SortBy::Ram | SortBy::RamAvg | SortBy::RamEnd => node.subtree_ram_bytes as f64,
        SortBy::Read | SortBy::Write | SortBy::Io | SortBy::IoAvg => {
            node.subtree_disk_io_delta_bytes as f64
        }
        SortBy::Pid => u32::MAX.saturating_sub(node.pid) as f64,
        SortBy::Name | SortBy::User | SortBy::Started | SortBy::Exited => {
            node.subtree_cpu_percent as f64
        }
    }
}

fn render_tree(report: &TreeReport, limit: usize, raw_bytes: bool, normalize_cpu: bool) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "Process tree: matched {} of {} processes\n",
        report.matched_processes, report.process_total
    ));
    output.push_str("PID      CPU%    SUBCPU% RAM       SUBRAM    USER       PROCESS\n");

    let mut printed = 0;
    for node in &report.nodes {
        render_node(
            node,
            "",
            limit,
            raw_bytes,
            normalize_cpu,
            report.logical_cpu_count,
            &mut printed,
            &mut output,
        );
        if printed >= limit {
            break;
        }
    }
    if printed == 0 {
        output.push_str("no matching processes\n");
    }
    output
}

#[allow(clippy::too_many_arguments)]
fn render_node(
    node: &TreeNode,
    prefix: &str,
    limit: usize,
    raw_bytes: bool,
    normalize_cpu: bool,
    logical_cpu_count: usize,
    printed: &mut usize,
    output: &mut String,
) {
    if *printed >= limit {
        return;
    }
    *printed += 1;
    let divisor = if normalize_cpu {
        logical_cpu_count.max(1) as f32
    } else {
        1.0
    };
    output.push_str(&format!(
        "{:<8} {:>6.1} {:>8.1} {:>9} {:>9} {:<10} {}{}\n",
        node.pid,
        node.cpu_percent / divisor,
        node.subtree_cpu_percent / divisor,
        format_bytes(node.ram_bytes, raw_bytes),
        format_bytes(node.subtree_ram_bytes, raw_bytes),
        truncate(&node.user_name, 10),
        prefix,
        node.display_name
    ));
    let child_prefix = format!("{prefix}  ");
    for child in &node.children {
        render_node(
            child,
            &child_prefix,
            limit,
            raw_bytes,
            normalize_cpu,
            logical_cpu_count,
            printed,
            output,
        );
    }
}

fn write_tree_csv(path: &std::path::Path, report: &TreeReport) -> Result<()> {
    let mut writer: Box<dyn std::io::Write> = if path == std::path::Path::new("-") {
        Box::new(std::io::stdout().lock())
    } else {
        Box::new(std::fs::File::create(path)?)
    };
    let mut csv = ::csv::Writer::from_writer(&mut writer);
    csv.write_record([
        "pid",
        "parent_pid",
        "depth",
        "name",
        "display_name",
        "user_name",
        "cpu_percent",
        "subtree_cpu_percent",
        "ram_bytes",
        "subtree_ram_bytes",
        "disk_io_delta_bytes",
        "subtree_disk_io_delta_bytes",
        "status",
        "run_time_seconds",
        "thread_count",
        "open_file_count",
        "cgroup_path",
    ])?;
    for node in &report.nodes {
        write_tree_csv_node(&mut csv, node, 0)?;
    }
    csv.flush()?;
    Ok(())
}

fn write_tree_csv_node<W: std::io::Write>(
    csv: &mut ::csv::Writer<W>,
    node: &TreeNode,
    depth: usize,
) -> Result<()> {
    csv.write_record([
        node.pid.to_string(),
        node.parent_pid
            .map(|pid| pid.to_string())
            .unwrap_or_default(),
        depth.to_string(),
        node.name.clone(),
        node.display_name.clone(),
        node.user_name.clone(),
        node.cpu_percent.to_string(),
        node.subtree_cpu_percent.to_string(),
        node.ram_bytes.to_string(),
        node.subtree_ram_bytes.to_string(),
        node.disk_io_delta_bytes.to_string(),
        node.subtree_disk_io_delta_bytes.to_string(),
        node.details.status.clone().unwrap_or_default(),
        node.details
            .run_time_seconds
            .map(|value| value.to_string())
            .unwrap_or_default(),
        node.details
            .thread_count
            .map(|value| value.to_string())
            .unwrap_or_default(),
        node.details
            .open_file_count
            .map(|value| value.to_string())
            .unwrap_or_default(),
        node.details.cgroup_path.clone().unwrap_or_default(),
    ])?;
    for child in &node.children {
        write_tree_csv_node(csv, child, depth + 1)?;
    }
    Ok(())
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        let mut output = value
            .chars()
            .take(max.saturating_sub(3))
            .collect::<String>();
        output.push_str("...");
        output
    }
}
