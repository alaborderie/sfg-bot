use std::sync::Arc;

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use crate::error::{VerifyResult, err};

pub const BOT_TOKEN: &str = "verify-not-a-discord-token";
pub const INTERACTION_TOKEN: &str = "verify-interaction-token";
pub const APP_ID: u64 = 900_000_000_000_000_001;
pub const USER_ID: u64 = 900_000_000_000_000_002;
pub const GUILD_ID: u64 = 900_000_000_000_000_003;
pub const CHANNEL_ID: u64 = 900_000_000_000_000_004;
pub const INTERACTION_ID: u64 = 900_000_000_000_000_010;
pub const COMMAND_ID: u64 = 900_000_000_000_000_020;

#[derive(Debug, Clone)]
pub struct CapturedRequest {
    pub method: String,
    pub path: String,
    pub body: Value,
}

pub struct DiscordStandIn {
    pub base_url: String,
    records: Arc<Mutex<Vec<CapturedRequest>>>,
}

impl DiscordStandIn {
    pub async fn start() -> VerifyResult<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|error| {
            err(format!(
                "failed to bind the local Discord stand-in: {error}"
            ))
        })?;
        let port = listener
            .local_addr()
            .map_err(|error| err(error.to_string()))?
            .port();
        let records = Arc::new(Mutex::new(Vec::new()));
        let accepted = Arc::clone(&records);
        tokio::spawn(async move {
            loop {
                let Ok((socket, _)) = listener.accept().await else {
                    break;
                };
                let accepted = Arc::clone(&accepted);
                tokio::spawn(async move {
                    if let Err(error) = handle_connection(socket, accepted).await {
                        tracing::debug!(error = %error, "discord stand-in connection failed");
                    }
                });
            }
        });
        Ok(Self {
            base_url: format!("http://127.0.0.1:{port}"),
            records,
        })
    }

    pub async fn captured(&self) -> Vec<CapturedRequest> {
        self.records.lock().await.clone()
    }
}

async fn handle_connection(
    mut socket: tokio::net::TcpStream,
    records: Arc<Mutex<Vec<CapturedRequest>>>,
) -> VerifyResult<()> {
    let (method, path, body) = read_request(&mut socket).await?;
    let parsed = serde_json::from_slice::<Value>(&body)
        .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&body).to_string() }));
    records.lock().await.push(CapturedRequest {
        method: method.clone(),
        path: path.clone(),
        body: parsed,
    });

    if path.contains("/interactions/") && path.contains("/callback") {
        write_response(&mut socket, "204 No Content", b"").await?;
    } else if path.contains("/webhooks/") {
        let message = stub_message();
        write_response(&mut socket, "200 OK", message.as_bytes()).await?;
    } else {
        let body = br#"{"message":"sfg-verify discord stand-in: unhandled route","code":0}"#;
        write_response(&mut socket, "404 Not Found", body).await?;
    }
    Ok(())
}

async fn read_request(
    socket: &mut tokio::net::TcpStream,
) -> VerifyResult<(String, String, Vec<u8>)> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    let header_end = loop {
        let n = socket.read(&mut tmp).await?;
        if n == 0 {
            return Err(err("Discord stand-in client closed before sending headers"));
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_subsequence(&buf, b"\r\n\r\n") {
            break pos;
        }
        if buf.len() > 2 * 1024 * 1024 {
            return Err(err("Discord stand-in request headers exceeded 2 MiB"));
        }
    };
    let header = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let content_length = content_length(&header);
    let mut body = buf[header_end + 4..].to_vec();
    while body.len() < content_length {
        let n = socket.read(&mut tmp).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }
    body.truncate(content_length);
    let request_line = header.lines().next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    if method.is_empty() || path.is_empty() {
        return Err(err(format!(
            "Discord stand-in could not parse the request line: {request_line}"
        )));
    }
    Ok((method, path, body))
}

fn content_length(header: &str) -> usize {
    header
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

async fn write_response(
    socket: &mut tokio::net::TcpStream,
    status: &str,
    body: &[u8],
) -> VerifyResult<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    socket.write_all(header.as_bytes()).await?;
    socket.write_all(body).await?;
    let _ = socket.shutdown().await;
    Ok(())
}

pub fn stub_message() -> String {
    r#"{"id":"900000000000000099","channel_id":"900000000000000004","author":{"id":"900000000000000001","username":"sfg-verify","global_name":null,"avatar":null,"bot":true,"banner":null,"accent_color":null,"locale":null,"verified":null,"email":null,"public_flags":null,"member":null,"primary_guild":null,"avatar_decoration_data":null,"collectibles":null},"content":"","timestamp":"2026-01-01T00:00:00Z","edited_timestamp":null,"tts":false,"mention_everyone":false,"mentions":[],"mention_roles":[],"attachments":[],"embeds":[],"pinned":false,"webhook_id":null,"type":0,"activity":null,"application":null,"application_id":"900000000000000001","message_reference":null,"flags":null,"referenced_message":null,"interaction":null,"interaction_metadata":null,"thread":null,"position":null,"role_subscription_data":null,"guild_id":"900000000000000003","member":null,"poll":null}"#.to_string()
}

pub fn visible_text(body: &Value) -> Vec<String> {
    let mut out = Vec::new();
    collect_text(body, &mut out);
    out
}

fn collect_text(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let Some(content) = map.get("content").and_then(Value::as_str)
                && !content.is_empty()
            {
                out.push(content.to_string());
            }
            if let Some(embeds) = map.get("embeds").and_then(Value::as_array) {
                for embed in embeds {
                    push_embed(embed, out);
                }
            }
            if let Some(embed) = map.get("embed") {
                push_embed(embed, out);
            }
            if let Some(data) = map.get("data") {
                collect_text(data, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_text(item, out);
            }
        }
        _ => {}
    }
}

fn push_embed(embed: &Value, out: &mut Vec<String>) {
    let Some(map) = embed.as_object() else {
        return;
    };
    for key in ["title", "description"] {
        if let Some(text) = map.get(key).and_then(Value::as_str)
            && !text.is_empty()
        {
            out.push(text.to_string());
        }
    }
    if let Some(fields) = map.get("fields").and_then(Value::as_array) {
        for field in fields {
            if let Some(name) = field.get("name").and_then(Value::as_str) {
                out.push(name.to_string());
            }
            if let Some(value) = field.get("value").and_then(Value::as_str) {
                out.push(value.to_string());
            }
        }
    }
}

pub fn interaction_json(
    command_name: &str,
    summoner_name: Option<&str>,
    guild_id: Option<u64>,
    channel_id: u64,
) -> Value {
    let mut options = Vec::new();
    if let Some(name) = summoner_name {
        options.push(json!({
            "name": "summoner_name",
            "type": 3,
            "value": name
        }));
    }
    json!({
        "id": INTERACTION_ID.to_string(),
        "application_id": APP_ID.to_string(),
        "data": {
            "id": COMMAND_ID.to_string(),
            "name": command_name,
            "type": 1,
            "options": options,
            "target_id": null
        },
        "guild_id": guild_id.map(|id| id.to_string()),
        "channel": null,
        "channel_id": channel_id.to_string(),
        "token": INTERACTION_TOKEN,
        "version": 1,
        "app_permissions": "0",
        "locale": "fr",
        "guild_locale": "fr",
        "entitlements": [],
        "context": null,
        "attachment_size_limit": 10_485_760,
        "user": {
            "id": USER_ID.to_string(),
            "username": "verify-user",
            "global_name": "Verify User",
            "avatar": null,
            "banner": null,
            "accent_color": null,
            "locale": "fr",
            "verified": null,
            "email": null,
            "public_flags": null,
            "member": null,
            "primary_guild": null,
            "avatar_decoration_data": null,
            "collectibles": null
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serenity::model::application::CommandInteraction;
    use serenity::model::channel::Message;

    #[test]
    fn stub_message_deserializes() {
        let message: Message = serde_json::from_str(&stub_message()).expect("message");
        assert_eq!(message.content, "");
    }

    #[test]
    fn interaction_json_deserializes() {
        let value = interaction_json("list-summoners", None, Some(GUILD_ID), CHANNEL_ID);
        let interaction: CommandInteraction = serde_json::from_value(value).expect("interaction");
        assert_eq!(interaction.data.name, "list-summoners");
        assert!(interaction.guild_id.is_some());
    }
}
