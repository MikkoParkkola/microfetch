//! Example showing how to use Arena allocator for response buffering
//!
//! Run with: `cargo run --example arena_usage`

use nab::arena::{Arena, ResponseBuffer};

fn main() {
    example_header_parsing();
    example_html_chunks();
    example_arena_reuse();
    example_memory_stats();
}

/// Simulate HTTP header parsing
fn example_header_parsing() {
    println!("=== HTTP Header Parsing ===");

    let arena = Arena::new();
    let mut buffer = ResponseBuffer::new(&arena);

    // Simulate parsing headers one at a time
    let headers = vec![
        "HTTP/1.1 200 OK",
        "Content-Type: text/html; charset=utf-8",
        "Content-Length: 12345",
        "Cache-Control: max-age=3600",
        "Set-Cookie: session=abc123",
    ];

    for header in headers {
        buffer.push_str(header);
        buffer.push_str("\r\n");
    }

    buffer.push_str("\r\n"); // Empty line separating headers from body

    let result = buffer.as_str();
    println!("Parsed {} bytes in {} parts", result.len(), buffer.part_count());
    println!("First 100 chars: {:?}\n", &result[..100.min(result.len())]);
}

/// Simulate HTML chunk processing
fn example_html_chunks() {
    println!("=== HTML Chunk Processing ===");

    let arena = Arena::new();
    let mut buffer = ResponseBuffer::new(&arena);

    // Simulate receiving HTML in chunks from network
    let chunks = vec![
        "<!DOCTYPE html><html>",
        "<head><title>Example</title></head>",
        "<body>",
        "<h1>Welcome</h1>",
        "<p>Content paragraph</p>",
        "</body></html>",
    ];

    for chunk in chunks {
        buffer.push_str(chunk);
    }

    let html = buffer.as_str();
    println!("Assembled {} bytes from {} chunks", html.len(), buffer.part_count());
    println!("HTML: {}\n", html);
}

/// Demonstrate arena reuse for multiple requests
fn example_arena_reuse() {
    println!("=== Arena Reuse Pattern ===");

    let mut arena = Arena::new();

    for request_num in 1..=3 {
        let mut buffer = ResponseBuffer::new(&arena);

        // Simulate processing different requests
        buffer.push_str(&format!("Request #{request_num}\n"));
        buffer.push_str("Status: 200 OK\n");
        buffer.push_str("Body: Some response data\n");

        let result = buffer.as_str();
        println!("Request {}: {} bytes", request_num, result.len());

        // Reset arena for next request (reuses memory)
        arena.reset();
    }
    println!();
}

/// Show memory statistics
fn example_memory_stats() {
    println!("=== Memory Statistics ===");

    let arena = Arena::with_chunk_size(8192); // 8KB chunks
    let mut buffer = ResponseBuffer::new(&arena);

    // Allocate some data
    for i in 0..100 {
        buffer.push_str(&format!("Line {i}: Some data here\n"));
    }

    println!("Chunks used: {}", arena.chunk_count());
    println!("Bytes allocated: {} ({} KB)", arena.bytes_allocated(), arena.bytes_allocated() / 1024);
    println!("Bytes used: {} ({} KB)", arena.bytes_used(), arena.bytes_used() / 1024);
    println!("Overhead: {:.1}%",
        (arena.bytes_allocated() - arena.bytes_used()) as f64 / arena.bytes_allocated() as f64 * 100.0
    );
    println!("Content size: {} bytes", buffer.len());
}
