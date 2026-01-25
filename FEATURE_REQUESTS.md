# microfetch Feature Requests for Auth Flows

## Background

To fully automate magic link authentication (Folo, etc.), microfetch needed three capabilities.

## 1. POST Request Support

**Status**: ✅ IMPLEMENTED (2026-01-25)
**Priority**: High

```bash
# Needed for magic link request
microfetch fetch URL --method POST --data '{"email":"x@y.com"}' --content-type application/json
```

**Use case**: Trigger magic link, API authentication, form submission

## 2. Cookie Capture from Response

**Status**: ✅ IMPLEMENTED (2026-01-25)
**Priority**: High

The internal reqwest cookie store captures Set-Cookie headers, but they're not exposed.

```bash
# Option A: Output all cookies after request chain
microfetch fetch URL --output-cookies
# Returns: name=value; name2=value2

# Option B: JSON format with cookie details
microfetch fetch URL --output-cookies --format json
# Returns: [{"name":"session_token","value":"xxx","domain":"folo.is",...}]
```

**Use case**: Extract session token set during redirect chain

## 3. Redirect Control

**Status**: ✅ IMPLEMENTED (2026-01-25)
**Priority**: Medium

```bash
# Don't follow redirects - capture 302 response and Set-Cookie
microfetch fetch URL --no-redirect

# Or limit redirects
microfetch fetch URL --max-redirects 0
```

**Use case**: Inspect redirect response before following, capture intermediate cookies

---

## Implementation Notes

### For cookie capture (fastest to implement):

In `http_client.rs`, the client already has `.cookie_store(true)`. The cookie jar is internal to reqwest but can be accessed:

```rust
// Option 1: Build with explicit cookie jar
use reqwest::cookie::Jar;
let jar = Arc::new(Jar::default());
let client = Client::builder()
    .cookie_provider(jar.clone())
    .build()?;

// After request, iterate jar
for cookie in jar.cookies(&url) {
    println!("{}", cookie);
}
```

### For POST support:

```rust
// In cmd_fetch, add method parameter
let request = match method.to_uppercase().as_str() {
    "POST" => client.inner().post(url).body(data),
    "PUT" => client.inner().put(url).body(data),
    _ => client.inner().get(url),
};
```

### For redirect control:

```rust
// In AcceleratedClient, make redirect policy configurable
pub fn with_max_redirects(self, max: usize) -> Self {
    // Rebuild client with new policy
}
```

---

## Workaround Until Implemented

For Folo magic link flow:

1. **Trigger magic link**: Use `requests` Python library (POST support)
2. **Read email**: Use `mcp-cli gmail/search_emails` or IMAP
3. **Extract cookie**: Click link in browser, then use `microfetch fetch --cookies brave`

This works but requires manual email click or browser_cookie3 fallback.

---

## ROI Analysis

| Feature | Effort | Value | Services Enabled |
|---------|--------|-------|------------------|
| POST support | 2h | High | All form-based auth, APIs |
| Cookie capture | 1h | High | Magic link, OAuth flows |
| Redirect control | 1h | Medium | OAuth, debugging |

**Total**: ~4h implementation for full auth automation
**Value**: $0/solve for any cookie-based auth (vs $0.003/solve for captcha services)
