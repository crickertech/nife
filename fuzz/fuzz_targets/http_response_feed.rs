//! Fuzz the package fetch's HTTP response reader with arbitrary bytes, cut into arbitrary reads.
//!
//! **Why this target exists.** `crates/http_response` runs in the progenitor on bytes that came off
//! the network, and it runs **before** the package digest check: nothing has vouched for a byte it
//! reads. The digest decides whether a package may run, but it cannot stop a head that panics the
//! reader, and a panic in the progenitor is the prompt going away. notes/packages.md recorded the
//! missing target as a BUGS entry when milestone 198 (a package manager, and the trivial install
//! that makes a second customer possible) landed the reader in its rung 3a.
//!
//! **Two properties, not only "it returned".** notes/fuzzing.md's BUGS section points out that three
//! of the older targets assert nothing, and that the one with a real property is the one that found a
//! silent bug. This reader has a property that is cheap to state and easy to get wrong:
//!
//! 1. **How the reads fall does not change the answer.** A socket hands back whatever it has, so the
//!    same response cut at different places must give the same status, the same body bytes and the
//!    same verdict. The first input byte picks the read size, and the rest is fed once whole and
//!    once in reads of that size. An error must be the same error either way, except that the
//!    split reader may already have handed back some body before a `BodyTooLong`, which is what the
//!    crate documents.
//! 2. **The body is never more than was declared, and is always the tail of the read it came in.**
//!    A body slice that is not a suffix of its own input would mean the head leaked into the body,
//!    and a hash over it would be a hash over the wrong bytes.
//!
//! The request writer, `get_request`, is exercised as well: whatever host and path the fuzzer picks,
//! a request the writer accepts has exactly one blank line, at its end, so no header can be smuggled.

#![no_main]

use http_response::{Error, Response, get_request};
use libfuzzer_sys::fuzz_target;

/// Feed `bytes` in reads of `step`, checking property 2 on every read, and return the outcome.
fn read_in(bytes: &[u8], step: usize) -> (Result<(), Error>, Response, Vec<u8>) {
    let mut response = Response::new();
    let mut body = Vec::new();
    for chunk in bytes.chunks(step) {
        match response.feed(chunk) {
            Ok(part) => {
                assert!(
                    chunk.ends_with(part),
                    "the body is not the tail of its read"
                );
                body.extend_from_slice(part);
                if let Some(length) = response.content_length() {
                    assert!(body.len() as u64 <= length, "more body than was declared");
                }
            }
            Err(e) => return (Err(e), response, body),
        }
    }
    (Ok(()), response, body)
}

fuzz_target!(|data: &[u8]| {
    let Some((&first, bytes)) = data.split_first() else {
        return;
    };
    let step = usize::from(first).max(1);

    let (whole, whole_response, whole_body) = read_in(bytes, bytes.len().max(1));
    let (split, split_response, split_body) = read_in(bytes, step);

    assert_eq!(whole, split, "the verdict depends on how the reads fell");
    assert_eq!(whole_response.status(), split_response.status());
    assert_eq!(
        whole_response.content_length(),
        split_response.content_length()
    );
    match whole {
        Ok(()) => {
            assert_eq!(
                whole_body, split_body,
                "the body depends on how the reads fell"
            );
            assert_eq!(whole_response.is_complete(), split_response.is_complete());
            if whole_response.is_complete() {
                assert_eq!(
                    Some(whole_body.len() as u64),
                    whole_response.content_length()
                );
            }
        }
        // The split reader may have handed back body before the read that overran; the whole
        // reader refuses that read in one piece. Anything handed back is still a prefix of the body.
        Err(Error::BodyTooLong) => assert!(whole_body.is_empty()),
        Err(_) => {
            assert!(whole_body.is_empty() && split_body.is_empty());
        }
    }

    // The request writer: split the same bytes into a host and a path at the step.
    let cut = step.min(bytes.len());
    if let (Ok(host), Ok(path)) = (
        core::str::from_utf8(&bytes[..cut]),
        core::str::from_utf8(&bytes[cut..]),
    ) {
        let mut out = [0u8; 512];
        if let Some(n) = get_request(host, path, &mut out) {
            let request = &out[..n];
            let blank = request.windows(4).position(|w| w == b"\r\n\r\n");
            assert_eq!(blank, Some(n - 4), "a request carried a second blank line");
            assert_eq!(
                request.windows(2).filter(|w| *w == b"\r\n").count(),
                3,
                "a request carried a line it was not asked for"
            );
        }
    }
});
