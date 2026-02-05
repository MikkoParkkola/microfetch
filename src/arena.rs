//! Arena Allocator for Response Buffering
//!
//! Optimized for HTTP response handling where many small strings (headers, HTML chunks,
//! markdown segments) are allocated and freed together.
//!
//! # Design
//!
//! - Simple bump allocator with 64KB chunks (default)
//! - Allocations within a single arena are freed all at once
//! - Per-request lifecycle: create arena, process response, drop arena
//! - Not thread-safe by design (arena is per-request, single-threaded)
//!
//! # Performance Characteristics
//!
//! **Best for:**
//! - Many small allocations (10-500 bytes each)
//! - All allocations freed together (request lifecycle)
//! - Reducing allocator overhead vs individual `String` allocations
//! - Predictable memory usage patterns
//!
//! **Not optimal for:**
//! - Long-lived data beyond request scope
//! - Single large allocations (use `String` with pre-allocated capacity)
//! - When `String::push_str` is sufficient (it's highly optimized)
//!
//! **Benchmark results (release mode):**
//! - Memory overhead: ~14.5% (chunk granularity)
//! - Best gains: 1000+ small allocations with batch deallocation
//! - Use `cargo bench --bench arena_benchmark` to measure on your workload
//!
//! # Example
//!
//! ```rust
//! use nab::arena::{Arena, ResponseBuffer};
//!
//! let arena = Arena::new();
//! let mut buffer = ResponseBuffer::new(&arena);
//!
//! buffer.push_str("HTTP/1.1 200 OK\r\n");
//! buffer.push_str("Content-Type: text/html\r\n");
//! buffer.push_str("\r\n<html>...</html>");
//!
//! let content = buffer.as_str();
//! assert!(content.contains("HTTP/1.1 200 OK"));
//! // Arena and all allocations freed here
//! ```

use std::cell::{Cell, RefCell};

/// Default chunk size for arena allocations (64KB)
const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

/// Arena allocator for temporary string storage
///
/// All allocations are freed when the arena is dropped.
pub struct Arena {
    chunks: RefCell<Vec<Vec<u8>>>,
    current_chunk_idx: Cell<usize>,
    current_offset: Cell<usize>,
    chunk_size: usize,
}

impl Arena {
    /// Create a new arena with default chunk size (64KB)
    #[must_use]
    pub fn new() -> Self {
        Self::with_chunk_size(DEFAULT_CHUNK_SIZE)
    }

    /// Create arena with custom chunk size
    #[must_use]
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        let mut chunks = Vec::new();
        chunks.push(Vec::with_capacity(chunk_size));

        Self {
            chunks: RefCell::new(chunks),
            current_chunk_idx: Cell::new(0),
            current_offset: Cell::new(0),
            chunk_size,
        }
    }

    /// Allocate a string slice in the arena
    ///
    /// Returns a reference with lifetime tied to the arena.
    pub fn alloc_str(&self, s: &str) -> &str {
        let bytes = self.alloc_bytes(s.as_bytes());
        // SAFETY: Input was valid UTF-8, we just copied the bytes
        unsafe { std::str::from_utf8_unchecked(bytes) }
    }

    /// Allocate a byte slice in the arena
    ///
    /// Returns a reference with lifetime tied to the arena.
    pub fn alloc_bytes(&self, bytes: &[u8]) -> &[u8] {
        let len = bytes.len();
        if len == 0 {
            return &[];
        }

        // If allocation is larger than chunk size, allocate dedicated chunk
        if len > self.chunk_size {
            return self.alloc_large(bytes);
        }

        let current_idx = self.current_chunk_idx.get();
        let current_offset = self.current_offset.get();

        // Check if current chunk has space
        let chunks = self.chunks.borrow();
        let available = chunks[current_idx].capacity() - current_offset;

        if available >= len {
            // Fast path: fits in current chunk
            drop(chunks);
            self.alloc_in_current_chunk(bytes, current_idx, current_offset)
        } else {
            // Need new chunk
            drop(chunks);
            self.alloc_new_chunk(bytes)
        }
    }

    /// Allocate in current chunk (fast path)
    fn alloc_in_current_chunk(&self, bytes: &[u8], chunk_idx: usize, offset: usize) -> &[u8] {
        let mut chunks = self.chunks.borrow_mut();
        let chunk = &mut chunks[chunk_idx];

        // Extend chunk to accommodate new data
        let start = offset;
        let end = offset + bytes.len();

        // Resize if needed
        if chunk.len() < end {
            chunk.resize(end, 0);
        }

        chunk[start..end].copy_from_slice(bytes);

        self.current_offset.set(end);

        // SAFETY: We just wrote valid data at this location
        // The slice lifetime is tied to &self, which is correct
        unsafe {
            let ptr = chunk.as_ptr().add(start);
            std::slice::from_raw_parts(ptr, bytes.len())
        }
    }

    /// Allocate in a new chunk
    fn alloc_new_chunk(&self, bytes: &[u8]) -> &[u8] {
        let mut chunks = self.chunks.borrow_mut();

        // Create new chunk
        let mut new_chunk = Vec::with_capacity(self.chunk_size);
        new_chunk.extend_from_slice(bytes);

        chunks.push(new_chunk);
        let new_idx = chunks.len() - 1;

        self.current_chunk_idx.set(new_idx);
        self.current_offset.set(bytes.len());

        // SAFETY: We just allocated and wrote this data
        unsafe {
            let chunk = &chunks[new_idx];
            let ptr = chunk.as_ptr();
            std::slice::from_raw_parts(ptr, bytes.len())
        }
    }

    /// Allocate large block (bigger than chunk size)
    fn alloc_large(&self, bytes: &[u8]) -> &[u8] {
        let mut chunks = self.chunks.borrow_mut();

        // Create dedicated large chunk
        let mut large_chunk = Vec::with_capacity(bytes.len());
        large_chunk.extend_from_slice(bytes);

        chunks.push(large_chunk);
        let idx = chunks.len() - 1;

        // Don't update current_chunk_idx - keep using the previous chunk for small allocs

        // SAFETY: We just allocated this chunk
        unsafe {
            let chunk = &chunks[idx];
            let ptr = chunk.as_ptr();
            std::slice::from_raw_parts(ptr, bytes.len())
        }
    }

    /// Reset arena without freeing memory (for reuse)
    ///
    /// This invalidates all previously allocated references.
    pub fn reset(&mut self) {
        let mut chunks = self.chunks.borrow_mut();

        // Keep first chunk, clear others
        if chunks.len() > 1 {
            chunks.truncate(1);
        }

        // Clear first chunk but keep capacity
        if let Some(first) = chunks.first_mut() {
            first.clear();
        }

        self.current_chunk_idx.set(0);
        self.current_offset.set(0);
    }

    /// Get total bytes allocated (including capacity)
    #[must_use]
    pub fn bytes_allocated(&self) -> usize {
        self.chunks.borrow().iter().map(Vec::capacity).sum()
    }

    /// Get total bytes in use
    #[must_use]
    pub fn bytes_used(&self) -> usize {
        self.chunks.borrow().iter().map(Vec::len).sum()
    }

    /// Get number of chunks
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.chunks.borrow().len()
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: Arena is not thread-safe by design
// It should only be used within a single task/thread

/// Response buffer backed by an arena allocator
///
/// Accumulates strings efficiently without individual allocations.
pub struct ResponseBuffer<'arena> {
    arena: &'arena Arena,
    parts: Vec<&'arena str>,
}

impl<'arena> ResponseBuffer<'arena> {
    /// Create a new response buffer
    #[must_use]
    pub fn new(arena: &'arena Arena) -> Self {
        Self {
            arena,
            parts: Vec::new(),
        }
    }

    /// Create with expected capacity (number of string parts)
    #[must_use]
    pub fn with_capacity(arena: &'arena Arena, capacity: usize) -> Self {
        Self {
            arena,
            parts: Vec::with_capacity(capacity),
        }
    }

    /// Push a string into the buffer
    pub fn push_str(&mut self, s: &str) {
        if !s.is_empty() {
            let allocated = self.arena.alloc_str(s);
            self.parts.push(allocated);
        }
    }

    /// Push bytes into the buffer (must be valid UTF-8)
    ///
    /// # Panics
    ///
    /// Panics if bytes are not valid UTF-8.
    pub fn push_bytes(&mut self, bytes: &[u8]) {
        if !bytes.is_empty() {
            let s = std::str::from_utf8(bytes).expect("Invalid UTF-8");
            self.push_str(s);
        }
    }

    /// Get the concatenated content as a single string
    ///
    /// This performs one final allocation to join all parts.
    #[must_use]
    pub fn as_str(&self) -> String {
        self.parts.concat()
    }

    /// Get the total length of all parts
    #[must_use]
    pub fn len(&self) -> usize {
        self.parts.iter().map(|s| s.len()).sum()
    }

    /// Check if buffer is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// Get number of string parts
    #[must_use]
    pub fn part_count(&self) -> usize {
        self.parts.len()
    }

    /// Clear all parts (but keep arena allocations)
    pub fn clear(&mut self) {
        self.parts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_basic() {
        let arena = Arena::new();
        let s1 = arena.alloc_str("hello");
        let s2 = arena.alloc_str(" world");

        assert_eq!(s1, "hello");
        assert_eq!(s2, " world");
    }

    #[test]
    fn test_arena_empty() {
        let arena = Arena::new();
        let empty = arena.alloc_str("");
        assert_eq!(empty, "");
    }

    #[test]
    fn test_arena_large_allocation() {
        let arena = Arena::with_chunk_size(1024);
        let large_str = "x".repeat(2048);
        let allocated = arena.alloc_str(&large_str);

        assert_eq!(allocated.len(), 2048);
        assert_eq!(allocated, large_str);
    }

    #[test]
    fn test_arena_multiple_chunks() {
        let arena = Arena::with_chunk_size(64);

        // Allocate enough to span multiple chunks
        let mut strings = Vec::new();
        for i in 0..10 {
            let s = format!("string_{i}_with_some_content");
            strings.push(arena.alloc_str(&s));
        }

        assert!(arena.chunk_count() > 1);
        assert_eq!(strings[0], "string_0_with_some_content");
        assert_eq!(strings[9], "string_9_with_some_content");
    }

    #[test]
    fn test_arena_bytes() {
        let arena = Arena::new();
        let bytes = b"binary data";
        let allocated = arena.alloc_bytes(bytes);

        assert_eq!(allocated, bytes);
    }

    #[test]
    fn test_arena_stats() {
        let arena = Arena::with_chunk_size(1024);

        let initial_allocated = arena.bytes_allocated();
        assert_eq!(initial_allocated, 1024); // One chunk

        arena.alloc_str("test");
        assert!(arena.bytes_used() >= 4);
    }

    #[test]
    fn test_arena_reset() {
        let mut arena = Arena::new();

        arena.alloc_str("test1");
        arena.alloc_str("test2");

        let used_before = arena.bytes_used();
        assert!(used_before > 0);

        arena.reset();

        let used_after = arena.bytes_used();
        assert_eq!(used_after, 0);

        // Can still allocate after reset
        let s = arena.alloc_str("after reset");
        assert_eq!(s, "after reset");
    }

    #[test]
    fn test_response_buffer_basic() {
        let arena = Arena::new();
        let mut buffer = ResponseBuffer::new(&arena);

        buffer.push_str("HTTP/1.1 200 OK\r\n");
        buffer.push_str("Content-Type: text/html\r\n");
        buffer.push_str("\r\n");
        buffer.push_str("<html><body>Hello</body></html>");

        let content = buffer.as_str();
        assert!(content.contains("HTTP/1.1 200 OK"));
        assert!(content.contains("<html>"));
        assert_eq!(buffer.part_count(), 4);
    }

    #[test]
    fn test_response_buffer_empty_strings() {
        let arena = Arena::new();
        let mut buffer = ResponseBuffer::new(&arena);

        buffer.push_str("hello");
        buffer.push_str(""); // Empty - should not add part
        buffer.push_str("world");

        assert_eq!(buffer.part_count(), 2);
        assert_eq!(buffer.as_str(), "helloworld");
    }

    #[test]
    fn test_response_buffer_len() {
        let arena = Arena::new();
        let mut buffer = ResponseBuffer::new(&arena);

        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());

        buffer.push_str("test");
        assert_eq!(buffer.len(), 4);
        assert!(!buffer.is_empty());

        buffer.push_str(" data");
        assert_eq!(buffer.len(), 9);
    }

    #[test]
    fn test_response_buffer_clear() {
        let arena = Arena::new();
        let mut buffer = ResponseBuffer::new(&arena);

        buffer.push_str("test");
        buffer.push_str("data");
        assert_eq!(buffer.part_count(), 2);

        buffer.clear();
        assert_eq!(buffer.part_count(), 0);
        assert!(buffer.is_empty());

        // Can still use after clear
        buffer.push_str("new");
        assert_eq!(buffer.as_str(), "new");
    }

    #[test]
    fn test_response_buffer_capacity() {
        let arena = Arena::new();
        let buffer = ResponseBuffer::with_capacity(&arena, 10);

        assert_eq!(buffer.part_count(), 0);
        // Capacity doesn't affect part_count, just pre-allocates Vec
    }
}
