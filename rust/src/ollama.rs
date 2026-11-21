//! Minimal local Ollama client (std TcpStream + serde_json, no HTTP crate).
//! Only talks to a local server; the default is 127.0.0.1:11434.

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub const TOP_LOGPROBS: u32 = 20;

/// Request body for one greedy next-token call with logprobs (`raw: true`, no chat template).
pub fn generate_body(model: &str, prompt: &str, seed: u64) -> String {
    json!({
        "model": model,
        "prompt": prompt,
        "raw": true,
        "stream": false,
        "logprobs": true,
        "top_logprobs": TOP_LOGPROBS,
        "keep_alive": "10m",
        "options": {"temperature": 0, "seed": seed, "num_predict": 1}
    })
    .to_string()
}

/// Decode an HTTP/1.1 response: returns (status, body). Handles Content-Length and chunked bodies.
pub fn parse_http_response(raw: &[u8]) -> Result<(u16, Vec<u8>), String> {
    let sep = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("no header terminator")?;
    let head = std::str::from_utf8(&raw[..sep]).map_err(|e| e.to_string())?;
    let body = &raw[sep + 4..];
    let mut lines = head.split("\r\n");
    let status_line = lines.next().ok_or("empty response")?;
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .ok_or("no status code")?
        .parse()
        .map_err(|_| "bad status code")?;
    let mut chunked = false;
    let mut len: Option<usize> = None;
    for l in lines {
        let lower = l.to_ascii_lowercase();
        if lower.starts_with("transfer-encoding:") && lower.contains("chunked") {
            chunked = true;
        } else if let Some(v) = lower.strip_prefix("content-length:") {
            len = v.trim().parse().ok();
        }
    }
    if chunked {
        return Ok((status, dechunk(body)?));
    }
    Ok((
        status,
        match len {
            Some(n) if n <= body.len() => body[..n].to_vec(),
            Some(_) => return Err("truncated body".into()),
            None => body.to_vec(),
        },
    ))
}

/// Decode a chunked transfer-encoded body.
pub fn dechunk(mut b: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let eol = b.windows(2).position(|w| w == b"\r\n").ok_or("bad chunk")?;
        let size_str = std::str::from_utf8(&b[..eol]).map_err(|e| e.to_string())?;
        let size = usize::from_str_radix(size_str.split(';').next().unwrap_or("").trim(), 16)
            .map_err(|_| format!("bad chunk size {size_str:?}"))?;
        b = &b[eol + 2..];
        if size == 0 {
            return Ok(out);
        }
        if b.len() < size + 2 {
            return Err("truncated chunk".into());
        }
        out.extend_from_slice(&b[..size]);
        b = &b[size + 2..];
    }
}

/// One HTTP request to a local server. `body = None` sends a GET.
pub fn http(addr: &str, path: &str, body: Option<&str>) -> Result<Value, String> {
    let mut s = TcpStream::connect(addr).map_err(|e| format!("connect {addr}: {e}"))?;
    s.set_read_timeout(Some(Duration::from_secs(600))).ok();
    let req = match body {
        Some(b) => format!(
            "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{b}",
            b.len()
        ),
        None => format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"),
    };
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let (status, body) = parse_http_response(&raw)?;
    let v: Value = serde_json::from_slice(&body).map_err(|e| format!("json: {e}"))?;
    if status != 200 {
        return Err(format!("HTTP {status}: {v}"));
    }
    Ok(v)
}

/// The next-token evidence for one prompt.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub token: String,
    pub lp_true: f64,
    pub lp_false: f64,
    /// true if " True"/" False" was missing from the top-k and got the floor value.
    pub true_imputed: bool,
    pub false_imputed: bool,
}

impl Answer {
    pub fn says_true(&self) -> bool {
        self.lp_true > self.lp_false
    }
    pub fn diff(&self) -> f64 {
        self.lp_true - self.lp_false
    }
}

/// Extract logprob(True) and logprob(False) from a /api/generate response.
/// A token counts as "True" if it equals "True" after trimming whitespace (same for "False").
/// A missing token gets (lowest top-k logprob − 1), as preregistered.
pub fn parse_answer(v: &Value) -> Result<Answer, String> {
    let first = v["logprobs"]
        .as_array()
        .and_then(|a| a.first())
        .ok_or("no logprobs in response (Ollama too old?)")?;
    let top = first["top_logprobs"].as_array().ok_or("no top_logprobs")?;
    if top.is_empty() {
        return Err("empty top_logprobs".into());
    }
    let mut floor = f64::INFINITY;
    let (mut t, mut f) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for e in top {
        let lp = e["logprob"].as_f64().ok_or("logprob not a number")?;
        floor = floor.min(lp);
        match e["token"].as_str().map(str::trim) {
            Some("True") => t = t.max(lp),
            Some("False") => f = f.max(lp),
            _ => {}
        }
    }
    let (ti, fi) = (t == f64::NEG_INFINITY, f == f64::NEG_INFINITY);
    Ok(Answer {
        token: first["token"].as_str().unwrap_or("").to_string(),
        lp_true: if ti { floor - 1.0 } else { t },
        lp_false: if fi { floor - 1.0 } else { f },
        true_imputed: ti,
        false_imputed: fi,
    })
}

/// name -> (digest, parameter_size, quantization) from a /api/tags response.
pub fn parse_tags(v: &Value) -> Vec<(String, String, String, String)> {
    v["models"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|m| {
                    let s = |x: &Value| x.as_str().unwrap_or("").to_string();
                    (
                        s(&m["name"]),
                        s(&m["digest"]),
                        s(&m["details"]["parameter_size"]),
                        s(&m["details"]["quantization_level"]),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}
