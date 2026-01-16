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

### Get OTP Codes
```bash
microfetch otp github.com
```

### Validate All Features
```bash
microfetch validate
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
