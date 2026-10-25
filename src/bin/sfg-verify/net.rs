use std::collections::HashMap;
use std::path::Path;

use serde_json::{Value, json};

use crate::error::VerifyResult;

/// Sockets this process owns whose remote address is not loopback and not unconnected.
///
/// `/proc/net/tcp` is the whole network namespace, so this matches socket inodes from
/// `/proc/self/fd` only. A verification drive must not open a production connection.
pub fn non_loopback_sockets() -> VerifyResult<Vec<Value>> {
    let mut by_inode = HashMap::new();
    read_tcp_table("/proc/net/tcp", false, &mut by_inode)?;
    if Path::new("/proc/net/tcp6").exists() {
        read_tcp_table("/proc/net/tcp6", true, &mut by_inode)?;
    }

    let mut hits = Vec::new();
    let fd_dir = Path::new("/proc/self/fd");
    let entries = std::fs::read_dir(fd_dir)?;
    for entry in entries {
        let entry = entry?;
        let Ok(target) = std::fs::read_link(entry.path()) else {
            continue;
        };
        let target = target.to_string_lossy();
        let Some(inode) = target
            .strip_prefix("socket:[")
            .and_then(|rest| rest.strip_suffix(']'))
            .and_then(|raw| raw.parse::<u64>().ok())
        else {
            continue;
        };
        let Some(remote) = by_inode.get(&inode) else {
            continue;
        };
        if remote.is_unconnected() || remote.is_loopback() {
            continue;
        }
        hits.push(json!({
            "inode": inode,
            "remote": remote.display(),
        }));
    }
    hits.sort_by(|a, b| {
        a.get("inode")
            .and_then(Value::as_u64)
            .cmp(&b.get("inode").and_then(Value::as_u64))
    });
    Ok(hits)
}

struct RemoteAddr {
    display: String,
    loopback: bool,
    unconnected: bool,
}

impl RemoteAddr {
    fn display(&self) -> &str {
        &self.display
    }

    fn is_loopback(&self) -> bool {
        self.loopback
    }

    fn is_unconnected(&self) -> bool {
        self.unconnected
    }
}

fn read_tcp_table(path: &str, ipv6: bool, into: &mut HashMap<u64, RemoteAddr>) -> VerifyResult<()> {
    let raw = std::fs::read_to_string(path)?;
    for line in raw.lines().skip(1) {
        let mut cols = line.split_whitespace();
        let _sl = cols.next();
        let _local = cols.next();
        let Some(remote) = cols.next() else {
            continue;
        };
        // st tx_queue:rx_queue tr:tm_when retrnsmt uid timeout inode
        let inode = cols.nth(6).and_then(|raw| raw.parse::<u64>().ok());
        let Some(inode) = inode else {
            continue;
        };
        into.insert(inode, parse_remote(remote, ipv6));
    }
    Ok(())
}

fn parse_remote(raw: &str, ipv6: bool) -> RemoteAddr {
    let Some((addr, port)) = raw.rsplit_once(':') else {
        return RemoteAddr {
            display: raw.to_string(),
            loopback: false,
            unconnected: false,
        };
    };
    let ip = if ipv6 {
        decode_ipv6(addr)
    } else {
        decode_ipv4(addr)
    };
    let unconnected = port == "0000" && (ip == "0.0.0.0" || ip == "::");
    let loopback = ip == "127.0.0.1" || ip == "::1";
    let port_num = u16::from_str_radix(port, 16).unwrap_or(0);
    RemoteAddr {
        display: format!("{ip}:{port_num}"),
        loopback,
        unconnected,
    }
}

fn decode_ipv4(hex_le: &str) -> String {
    let Ok(value) = u32::from_str_radix(hex_le, 16) else {
        return hex_le.to_string();
    };
    let bytes = value.to_le_bytes();
    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
}

fn decode_ipv6(hex_le: &str) -> String {
    if hex_le.len() != 32 {
        return hex_le.to_string();
    }
    let mut bytes = [0u8; 16];
    for (index, chunk) in hex_le.as_bytes().chunks(8).enumerate() {
        let word = std::str::from_utf8(chunk).unwrap_or("00000000");
        let value = u32::from_str_radix(word, 16).unwrap_or(0);
        let le = value.to_le_bytes();
        let start = index * 4;
        bytes[start..start + 4].copy_from_slice(&le);
    }
    if bytes == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1] {
        return "::1".to_string();
    }
    if bytes.iter().all(|byte| *byte == 0) {
        return "::".to_string();
    }
    // Enough to tell loopback from everything else. Show the raw bytes if it is not ::1.
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}
