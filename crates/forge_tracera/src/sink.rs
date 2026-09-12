//! The concrete [`TraceraSink`] — HTTP transport + auth + offline retry.

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::{AuthMode, SinkConfig};
use crate::error::{SinkError, SinkResult};
use crate::event::TraceraEvent;
use crate::store::StoreHandle;
use crate::TelemetrySink;

/// Wire envelope wrapping a batch of events.
#[derive(Debug, serde::Serialize)]
struct Batch<'a> {
    /// Events in submission order.
    events: &'a [TraceraEvent],
}

/// The default Tracera sink implementation.
///
/// Constructed via [`TraceraSink::new`] or [`TraceraSink::with_client` (for
/// tests that inject a custom HTTP client). All operations are async and
/// non-blocking from the caller's side — [`submit`](TelemetrySink::submit)
/// only enqueues into the in-memory store.
///
/// Delivery is driven by [`flush`](TelemetrySink::flush). On transient
/// failures (network error, 5xx, 429) the batch is re-queued and retried
/// with exponential back-off, up to `config.max_retries`.
pub struct TraceraSink {
    cfg: SinkConfig,
    client: Client,
    store: StoreHandle,
    shutdown: AtomicBool,
}

impl TraceraSink {
    /// Construct a sink with the given config. The config is validated;
    /// invalid configurations return [`SinkError::InvalidConfig`].
    pub fn new(cfg: SinkConfig) -> SinkResult<Self> {
        cfg.validate().map_err(SinkError::InvalidConfig)?;
        cfg.auth.validate().map_err(SinkError::InvalidAuth)?;
        let client = Client::builder()
            .timeout(cfg.request_timeout)
            .build()
            .map_err(|e| SinkError::Transport(e.to_string()))?;
        Ok(Self {
            store: StoreHandle::new(cfg.capacity),
            client,
            cfg,
            shutdown: AtomicBool::new(false),
        })
    }

    /// Construct a sink with an injected HTTP client (used in tests).
    pub fn with_client(cfg: SinkConfig, client: Client) -> SinkResult<Self> {
        cfg.validate().map_err(SinkError::InvalidConfig)?;
        cfg.auth.validate().map_err(SinkError::InvalidAuth)?;
        Ok(Self {
            store: StoreHandle::new(cfg.capacity),
            client,
            cfg,
            shutdown: AtomicBool::new(false),
        })
    }

    /// Handle to the underlying store (used in tests + observability hooks).
    pub fn store(&self) -> StoreHandle {
        self.store.clone()
    }

    /// Read-only view of the config.
    pub fn config(&self) -> &SinkConfig {
        &self.cfg
    }

    /// Compute the HMAC-SHA256 of `body` using `secret`, hex-encoded.
    fn hmac_signature(secret: &[u8], body: &[u8]) -> String {
        // Minimal HMAC-SHA256 (RFC 2104). Avoids adding a crypto dep just
        // for this — the SHA-256 block primitives are small and stable.
        hmac_sha256_hex(secret, body)
    }

    /// Send one batch with retry semantics. Returns Ok(()) when the
    /// server has accepted the batch.
    async fn send_batch_with_retry(&self, events: Vec<TraceraEvent>) -> SinkResult<()> {
        let mut backoff = self.cfg.initial_backoff;
        let mut last_err: Option<SinkError> = None;
        for attempt in 0..self.cfg.max_retries {
            if self.shutdown.load(Ordering::SeqCst) {
                return Err(SinkError::Shutdown);
            }
            match self.send_batch(&events).await {
                Ok(()) => return Ok(()),
                Err(SinkError::AuthRejected { .. }) | Err(SinkError::InvalidConfig(_)) => {
                    // Non-retryable
                    return Err(last_err.unwrap_or(SinkError::ServerRejected {
                        status: 0,
                        body: "unknown".to_string(),
                    }));
                }
                Err(e) => {
                    last_err = Some(e);
                    tracing::warn!(
                        attempt = attempt + 1,
                        max = self.cfg.max_retries,
                        backoff_ms = backoff.as_millis() as u64,
                        "tracera batch send failed; retrying"
                    );
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(self.cfg.max_backoff);
                }
            }
        }
        Err(last_err.unwrap_or(SinkError::Transport(
            "exhausted retries".to_string(),
        )))
    }

    /// Send one batch (no retry). On failure, returns the original
    /// `events` back to the caller via the re-queue path in `flush`.
    async fn send_batch(&self, events: &[TraceraEvent]) -> SinkResult<()> {
        let batch = Batch { events };
        let body = serde_json::to_vec(&batch)?;

        let mut req = self
            .client
            .post(&self.cfg.endpoint)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header("X-Tracera-Schema", crate::event::TRACERA_SCHEMA);

        // Auth
        match &self.cfg.auth {
            AuthMode::None => {}
            AuthMode::Bearer { token } => {
                req = req.bearer_auth(token);
            }
            AuthMode::Hmac { secret, header } => {
                let sig = Self::hmac_signature(secret.as_bytes(), &body);
                req = req.header(header.as_str(), sig);
            }
        }

        let resp = req
            .body(body)
            .send()
            .await
            .map_err(|e| SinkError::Transport(e.to_string()))?;

        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let body = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(512)
            .collect::<String>();
        match status {
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(SinkError::AuthRejected { status: status.as_u16() })
            }
            StatusCode::TOO_MANY_REQUESTS => Err(SinkError::ServerRejected {
                status: status.as_u16(),
                body,
            }),
            s if s.is_server_error() => Err(SinkError::ServerRejected {
                status: s.as_u16(),
                body,
            }),
            _ => Err(SinkError::ServerRejected {
                status: status.as_u16(),
                body,
            }),
        }
    }
}

#[async_trait]
impl TelemetrySink for TraceraSink {
    async fn submit(&self, event: TraceraEvent) -> SinkResult<()> {
        if self.shutdown.load(Ordering::SeqCst) {
            return Err(SinkError::Shutdown);
        }
        // Stamp source if not already set.
        let mut ev = event;
        if ev.source.is_empty() {
            ev.source = self.cfg.source.clone();
        }
        if ev.schema.is_empty() {
            ev.schema = crate::event::TRACERA_SCHEMA.to_string();
        }
        self.store
            .submit(ev, self.cfg.block_on_full)
            .map_err(|full| SinkError::StoreFull {
                capacity: full.capacity,
                dropped: self.store.total_dropped() as usize,
            })
    }

    async fn flush(&self) -> SinkResult<usize> {
        let mut delivered = 0usize;
        while !self.store.is_empty() {
            if self.shutdown.load(Ordering::SeqCst) {
                return Err(SinkError::Shutdown);
            }
            let batch = self.store.drain_batch(self.cfg.batch_size);
            if batch.is_empty() {
                break;
            }
            match self.send_batch_with_retry(batch.clone()).await {
                Ok(()) => delivered += batch.len(),
                Err(e) => {
                    // Re-queue and propagate. Caller can decide to drop or
                    // persist. We preserve ordering.
                    let _ = self.store.requeue_front(batch);
                    return Err(e);
                }
            }
        }
        Ok(delivered)
    }

    async fn shutdown(&self) -> SinkResult<usize> {
        self.shutdown.store(true, Ordering::SeqCst);
        let delivered = self.flush().await.unwrap_or(0);
        Ok(delivered)
    }
}

// ---------------------------------------------------------------------------
// Minimal HMAC-SHA256 (RFC 2104) — avoids pulling in a crypto dep.
// ---------------------------------------------------------------------------

const SHA256_BLOCK: usize = 64;
const SHA256_DIGEST: usize = 32;

fn sha256(msg: &[u8]) -> [u8; SHA256_DIGEST] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    // Pre-processing: pad with 1 bit, zeros, 64-bit length.
    let bit_len = (msg.len() as u64).wrapping_mul(8);
    let mut buf = msg.to_vec();
    buf.push(0x80);
    while buf.len() % 64 != 56 {
        buf.push(0);
    }
    buf.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in buf.chunks(SHA256_BLOCK) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let mj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(mj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; SHA256_DIGEST];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

fn hmac_sha256_hex(key: &[u8], msg: &[u8]) -> String {
    let mut k = if key.len() > SHA256_BLOCK {
        sha256(key).to_vec()
    } else {
        key.to_vec()
    };
    if k.len() < SHA256_BLOCK {
        k.resize(SHA256_BLOCK, 0);
    }
    let mut ipad = [0x36u8; SHA256_BLOCK];
    let mut opad = [0x5cu8; SHA256_BLOCK];
    for i in 0..SHA256_BLOCK {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }
    let mut inner = sha256(&[&ipad, msg].concat());
    let outer = sha256(&[&opad[..], &inner].concat());
    inner = outer;
    let mut s = String::with_capacity(64);
    for b in inner.iter() {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_sha256_matches_rfc4231_test_case_1() {
        // RFC 4231 §4.2: key = 0x0b * 20, data = "Hi There"
        // expected = b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7
        let key = vec![0x0b; 20];
        let data = b"Hi There";
        let hex = hmac_sha256_hex(&key, data);
        assert_eq!(
            hex,
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn hmac_sha256_matches_rfc4231_test_case_2() {
        // RFC 4231 §4.3: key = "Jefe", data = "what do ya want for nothing?"
        // expected = 5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843
        let hex = hmac_sha256_hex(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hex,
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn new_validates_config() {
        let bad = SinkConfig {
            endpoint: String::new(),
            ..SinkConfig::default()
        };
        assert!(matches!(
            TraceraSink::new(bad),
            Err(SinkError::InvalidConfig(_))
        ));
    }

    #[test]
    fn new_validates_auth() {
        let bad = SinkConfig {
            auth: AuthMode::Bearer {
                token: String::new(),
            },
            ..SinkConfig::default()
        };
        assert!(matches!(
            TraceraSink::new(bad),
            Err(SinkError::InvalidAuth(_))
        ));
    }

    #[tokio::test]
    async fn submit_returns_store_full_when_block_is_false() {
        let cfg = SinkConfig {
            capacity: 1,
            block_on_full: false,
            ..SinkConfig::default()
        };
        let sink = TraceraSink::new(cfg).unwrap();
        sink.submit(TraceraEvent::new(
            "forgecode",
            crate::event::EventKind::Prompt,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
        let r = sink
            .submit(TraceraEvent::new(
                "forgecode",
                crate::event::EventKind::Prompt,
                serde_json::json!({}),
            ))
            .await;
        assert!(matches!(r, Err(SinkError::StoreFull { .. })));
    }

    #[tokio::test]
    async fn submit_after_shutdown_returns_shutdown() {
        let sink = TraceraSink::new(SinkConfig::default()).unwrap();
        sink.shutdown().await.unwrap();
        let r = sink
            .submit(TraceraEvent::new(
                "forgecode",
                crate::event::EventKind::Prompt,
                serde_json::json!({}),
            ))
            .await;
        assert!(matches!(r, Err(SinkError::Shutdown)));
    }

    #[tokio::test]
    async fn flush_returns_zero_when_empty() {
        let sink = TraceraSink::new(SinkConfig::default()).unwrap();
        let n = sink.flush().await.unwrap();
        assert_eq!(n, 0);
    }
}