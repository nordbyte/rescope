use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result};
use rescope_core::SnapshotReport;

pub enum PrometheusSink {
    Stdout,
    File(PathBuf),
    Http(PrometheusServer),
}

impl PrometheusSink {
    pub fn new(target: &str) -> Result<Self> {
        if target == "-" {
            return Ok(Self::Stdout);
        }
        if is_http_target(target) {
            return Ok(Self::Http(PrometheusServer::bind(target)?));
        }
        Ok(Self::File(PathBuf::from(target)))
    }

    pub fn writes_stdout(target: &Option<String>) -> bool {
        target.as_deref() == Some("-")
    }

    pub fn publish(&mut self, report: &SnapshotReport) -> Result<()> {
        let text = format_snapshot(report);
        match self {
            Self::Stdout => {
                print!("{text}");
                std::io::stdout().flush()?;
            }
            Self::File(path) => {
                write_file(path, &text)?;
            }
            Self::Http(server) => server.publish(text),
        }
        Ok(())
    }
}

pub struct PrometheusServer {
    metrics: Arc<Mutex<String>>,
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl PrometheusServer {
    fn bind(addr: &str) -> Result<Self> {
        let listener = TcpListener::bind(addr).with_context(|| format!("binding {addr}"))?;
        listener.set_nonblocking(true)?;
        let metrics = Arc::new(Mutex::new(String::new()));
        let running = Arc::new(AtomicBool::new(true));
        let thread_metrics = Arc::clone(&metrics);
        let thread_running = Arc::clone(&running);
        let handle = thread::spawn(move || serve(listener, thread_metrics, thread_running));
        Ok(Self {
            metrics,
            running,
            handle: Some(handle),
        })
    }

    fn publish(&self, text: String) {
        if let Ok(mut metrics) = self.metrics.lock() {
            *metrics = text;
        }
    }
}

impl Drop for PrometheusServer {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn serve(listener: TcpListener, metrics: Arc<Mutex<String>>, running: Arc<AtomicBool>) {
    while running.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = handle_stream(&mut stream, &metrics);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(_) => thread::sleep(Duration::from_millis(100)),
        }
    }
}

fn handle_stream(stream: &mut TcpStream, metrics: &Arc<Mutex<String>>) -> std::io::Result<()> {
    let mut buffer = [0_u8; 1024];
    let bytes = stream.read(&mut buffer).unwrap_or_default();
    let request = String::from_utf8_lossy(&buffer[..bytes]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    if path != "/metrics" {
        stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")?;
        return Ok(());
    }

    let body = metrics
        .lock()
        .map(|metrics| metrics.clone())
        .unwrap_or_default();
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

pub fn format_snapshot(report: &SnapshotReport) -> String {
    let mut output = String::new();
    output.push_str("# HELP rescope_system_cpu_percent Global system CPU percentage.\n");
    output.push_str("# TYPE rescope_system_cpu_percent gauge\n");
    output.push_str(&format!(
        "rescope_system_cpu_percent {}\n",
        report.global_cpu_percent
    ));
    output.push_str("# HELP rescope_system_memory_bytes System memory by state.\n");
    output.push_str("# TYPE rescope_system_memory_bytes gauge\n");
    output.push_str(&format!(
        "rescope_system_memory_bytes{{state=\"total\"}} {}\n",
        report.total_memory_bytes
    ));
    output.push_str(&format!(
        "rescope_system_memory_bytes{{state=\"available\"}} {}\n",
        report.available_memory_bytes
    ));
    output
        .push_str("# HELP rescope_system_network_delta_bytes Network bytes in the last sample.\n");
    output.push_str("# TYPE rescope_system_network_delta_bytes gauge\n");
    output.push_str(&format!(
        "rescope_system_network_delta_bytes{{direction=\"received\"}} {}\n",
        report.network_received_delta_bytes
    ));
    output.push_str(&format!(
        "rescope_system_network_delta_bytes{{direction=\"transmitted\"}} {}\n",
        report.network_transmitted_delta_bytes
    ));
    output.push_str("# HELP rescope_process_cpu_percent Row CPU percentage.\n");
    output.push_str("# TYPE rescope_process_cpu_percent gauge\n");
    output.push_str("# HELP rescope_process_memory_bytes Row resident memory bytes.\n");
    output.push_str("# TYPE rescope_process_memory_bytes gauge\n");
    output
        .push_str("# HELP rescope_process_io_delta_bytes Row disk I/O bytes in the last sample.\n");
    output.push_str("# TYPE rescope_process_io_delta_bytes gauge\n");

    for row in &report.rows {
        let labels = labels(row.group_type, &row.display_name, row.pid);
        output.push_str(&format!(
            "rescope_process_cpu_percent{{{labels}}} {}\n",
            row.cpu_percent
        ));
        output.push_str(&format!(
            "rescope_process_memory_bytes{{{labels}}} {}\n",
            row.ram_bytes
        ));
        output.push_str(&format!(
            "rescope_process_io_delta_bytes{{{labels},direction=\"read\"}} {}\n",
            row.disk_read_delta_bytes
        ));
        output.push_str(&format!(
            "rescope_process_io_delta_bytes{{{labels},direction=\"write\"}} {}\n",
            row.disk_write_delta_bytes
        ));
    }

    output
}

fn labels(group_type: rescope_core::GroupBy, display_name: &str, pid: Option<u32>) -> String {
    let mut labels = format!(
        "group_type=\"{}\",display_name=\"{}\"",
        format!("{group_type:?}").to_ascii_lowercase(),
        escape_label(display_name)
    );
    if let Some(pid) = pid {
        labels.push_str(&format!(",pid=\"{pid}\""));
    }
    labels
}

fn escape_label(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn is_http_target(target: &str) -> bool {
    target.contains(':') && !target.contains(std::path::MAIN_SEPARATOR)
}

fn write_file(path: &Path, text: &str) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let mut temp_file = match parent {
        Some(parent) => tempfile::NamedTempFile::new_in(parent)?,
        None => tempfile::NamedTempFile::new_in(".")?,
    };
    temp_file.write_all(text.as_bytes())?;
    temp_file.as_file_mut().sync_all()?;
    temp_file.persist(path)?;
    Ok(())
}
