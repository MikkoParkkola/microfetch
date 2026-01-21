# MicroFetch

Ultra-minimal browser engine with HTTP/3, JS support, cookie auth, passkeys, and anti-fingerprinting.

## Features

- **HTTP Acceleration**: HTTP/2 multiplexing, HTTP/3 (QUIC) with 0-RTT, TLS 1.3, Brotli/Zstd compression
- **Browser Fingerprinting**: Realistic Chrome/Firefox/Safari profiles to avoid detection
- **Authentication**:
  - 1Password CLI integration
  - Apple Keychain password retrieval
  - Browser cookie extraction (Brave, Chrome, Firefox, Safari)
  - Browser password storage (Chromium-based)
- **JavaScript**: QuickJS engine with minimal DOM (ES2020 support)
- **WebSocket**: Full WebSocket support with JSON-RPC convenience layer
- **Prefetching**: Early Hints (103) support, link hint extraction

## Installation

```bash
cargo install --path .
```

## Usage

### Fetch a URL
```bash
microfetch fetch https://example.com

# With browser cookies
microfetch fetch https://example.com --cookies brave

# With 1Password credentials
microfetch fetch https://example.com --1password
```

### Benchmark
```bash
microfetch bench "https://example.com,https://httpbin.org/get" -i 10
```

### Generate Browser Fingerprints
```bash
microfetch fingerprint -c 5
```

### Test 1Password Integration
```bash
microfetch auth https://github.com
```

### Token-Optimized Output (LLM-friendly)
```bash
# Compact format: STATUS SIZE TIME
microfetch fetch https://api.example.com --format compact
# 200 1234B 45ms

# JSON format for parsing
microfetch fetch https://api.example.com --format json

# Save full body to file (bypasses truncation)
microfetch fetch https://example.com --output body.html

# Convert HTML to markdown
microfetch fetch https://example.com --markdown
```

### Custom Headers & Session Warmup
```bash
# Add custom headers (API access)
microfetch fetch https://api.example.com \
  --add-header "Accept: application/json" \
  --add-header "X-Custom: value"

# Auto-add Referer header
microfetch fetch https://api.example.com --auto-referer

# Warmup session first (for APIs requiring prior page load)
microfetch fetch https://api.example.com/data \
  --cookies brave \
  --warmup-url https://example.com/dashboard
```

### Get OTP Codes
```bash
microfetch otp github.com
```

### Validate All Features
```bash
microfetch validate
```

## Kauppalehti Portfolio (KL.fi)

For authenticated Kauppalehti portfolio access, use the dedicated helper which handles CloudFront session requirements:

```bash
# Compact format (token-optimized for LLMs)
kl-portfolio 471838 --format compact
# KL:Oma salkku|€490,950|+132.2%|2026-01-21T14:02
# HARVIA|3500|€143,850|-0.4%
# ...

# Full format with names
kl-portfolio 471838 --format full

# JSON for parsing
kl-portfolio 471838 --format json
```

## Library Usage

```rust
use microfetch::AcceleratedClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = AcceleratedClient::new()?;
    let html = client.fetch_text("https://example.com").await?;
    println!("Fetched {} bytes", html.len());
    Ok(())
}
```

## HTTP/3 Support

HTTP/3 is enabled by default. To disable:

```bash
cargo build --no-default-features --features cli
```

## License

MIT
