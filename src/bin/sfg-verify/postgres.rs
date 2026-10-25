use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{VerifyResult, err};

pub struct PgBins {
    pub initdb: PathBuf,
    pub pg_ctl: PathBuf,
    pub postgres: PathBuf,
}

pub fn find_bins() -> VerifyResult<PgBins> {
    let dir = if let Some(explicit) = std::env::var_os("SFG_VERIFY_PG_BIN") {
        PathBuf::from(explicit)
    } else {
        newest_pg_bin_dir()?
    };
    let bins = PgBins {
        initdb: dir.join("initdb"),
        pg_ctl: dir.join("pg_ctl"),
        postgres: dir.join("postgres"),
    };
    for path in [&bins.initdb, &bins.pg_ctl, &bins.postgres] {
        if !path.is_file() {
            return Err(err(format!(
                "PostgreSQL binary missing at {}. Install the server package (Debian/Ubuntu: `sudo apt-get install -y postgresql postgresql-client`) or set SFG_VERIFY_PG_BIN to the directory that contains initdb, pg_ctl, and postgres. The harness does not use a system cluster on port 5432.",
                path.display()
            )));
        }
    }
    Ok(bins)
}

fn newest_pg_bin_dir() -> VerifyResult<PathBuf> {
    let root = Path::new("/usr/lib/postgresql");
    if !root.is_dir() {
        return Err(err(
            "PostgreSQL is not installed under /usr/lib/postgresql. Install the server package (Debian/Ubuntu: `sudo apt-get install -y postgresql postgresql-client`) or set SFG_VERIFY_PG_BIN. The harness starts a private cluster and does not connect to a shared server.",
        ));
    }
    let mut versions = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.path().join("bin").join("initdb").is_file() {
            versions.push(entry.file_name());
        }
    }
    versions.sort();
    let Some(latest) = versions.last() else {
        return Err(err(
            "/usr/lib/postgresql has no version with bin/initdb. Install the postgresql server package or set SFG_VERIFY_PG_BIN.",
        ));
    };
    Ok(root.join(latest).join("bin"))
}

pub fn init_and_start(data_dir: &Path, port: u16, bins: &PgBins) -> VerifyResult<()> {
    if data_dir.exists() {
        return Err(err(format!(
            "postgres data directory {} already exists; refusing to reuse it",
            data_dir.display()
        )));
    }
    fs::create_dir_all(data_dir)?;
    let init = Command::new(&bins.initdb)
        .arg("-D")
        .arg(data_dir)
        .args([
            "--username=sfgverify",
            "--auth=trust",
            "--encoding=UTF8",
            "--locale=C",
            "--no-sync",
        ])
        .output()?;
    if !init.status.success() {
        return Err(err(format!(
            "initdb failed (exit {:?}). stdout:\n{}\nstderr:\n{}",
            init.status.code(),
            String::from_utf8_lossy(&init.stdout),
            String::from_utf8_lossy(&init.stderr)
        )));
    }

    let log_path = data_dir.parent().unwrap_or(data_dir).join("postgres.log");
    let options = format!(
        "-p {port} -k {} -c listen_addresses=127.0.0.1 -c unix_socket_directories={}",
        data_dir.display(),
        data_dir.display()
    );
    let start = Command::new(&bins.pg_ctl)
        .arg("-D")
        .arg(data_dir)
        .arg("-l")
        .arg(&log_path)
        .args(["-w", "-t", "30", "-o", &options, "start"])
        .output()?;
    if !start.status.success() {
        let log = fs::read_to_string(&log_path).unwrap_or_default();
        return Err(err(format!(
            "pg_ctl start failed (exit {:?}) for data dir {} on 127.0.0.1:{port}. stdout:\n{}\nstderr:\n{}\nlog:\n{log}",
            start.status.code(),
            data_dir.display(),
            String::from_utf8_lossy(&start.stdout),
            String::from_utf8_lossy(&start.stderr)
        )));
    }
    wait_ready(port)?;
    Ok(())
}

fn wait_ready(port: u16) -> VerifyResult<()> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut last = String::new();
    while Instant::now() < deadline {
        let output = Command::new("pg_isready")
            .args([
                "-h",
                "127.0.0.1",
                "-p",
                &port.to_string(),
                "-U",
                "sfgverify",
            ])
            .output();
        match output {
            Ok(output) if output.status.success() => return Ok(()),
            Ok(output) => {
                last = format!(
                    "{} {}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(error) => {
                return Err(err(format!(
                    "pg_isready is not on PATH ({error}). Install postgresql-client. The private cluster may still be running; run `sfg-verify cleanup` for this home."
                )));
            }
        }
        thread::sleep(Duration::from_millis(200));
    }
    Err(err(format!(
        "postgres on 127.0.0.1:{port} did not accept connections within 20s. Last pg_isready output: {last}"
    )))
}

pub fn create_database(port: u16) -> VerifyResult<()> {
    let output = Command::new("psql")
        .args([
            "-h",
            "127.0.0.1",
            "-p",
            &port.to_string(),
            "-U",
            "sfgverify",
            "-d",
            "postgres",
            "-v",
            "ON_ERROR_STOP=1",
            "-c",
            "CREATE DATABASE sfg_verify",
        ])
        .output()
        .map_err(|error| {
            err(format!(
                "psql is not on PATH ({error}). Install postgresql-client."
            ))
        })?;
    if !output.status.success() {
        return Err(err(format!(
            "CREATE DATABASE sfg_verify failed (exit {:?}). stdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

pub fn postmaster_pid(data_dir: &Path) -> VerifyResult<u32> {
    let raw = fs::read_to_string(data_dir.join("postmaster.pid")).map_err(|_| {
        err(format!(
            "postgres data dir {} has no postmaster.pid; the private cluster is not running",
            data_dir.display()
        ))
    })?;
    let pid = raw.lines().next().unwrap_or("").trim();
    pid.parse::<u32>().map_err(|_| {
        err(format!(
            "postmaster.pid in {} does not start with a pid (first line: {pid})",
            data_dir.display()
        ))
    })
}

pub fn pid_owns_data_dir(pid: u32, data_dir: &Path) -> VerifyResult<bool> {
    let cmdline = fs::read(format!("/proc/{pid}/cmdline")).map_err(|_| {
        err(format!(
            "postmaster pid {pid} is not running; the verification cluster is down"
        ))
    })?;
    let rendered = String::from_utf8_lossy(&cmdline).replace('\0', " ");
    let data = data_dir.to_string_lossy();
    if !rendered.contains("postgres") || !rendered.contains(data.as_ref()) {
        return Ok(false);
    }
    Ok(true)
}

pub fn stop(data_dir: &Path, bins: &PgBins) -> VerifyResult<()> {
    let output = Command::new(&bins.pg_ctl)
        .arg("-D")
        .arg(data_dir)
        .args(["-m", "fast", "-w", "-t", "30", "stop"])
        .output()?;
    if !output.status.success() {
        return Err(err(format!(
            "pg_ctl stop failed for {} (exit {:?}). stdout:\n{}\nstderr:\n{}",
            data_dir.display(),
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}
