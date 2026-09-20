//! SHA-256 and SHA-384 behind `rustls`' `Hash` trait.
//!
//! Nothing here computes a hash. `sha2` does, and it is taken rather than written for the reason
//! in DECISIONS §46 (thin primitives or whole subsystems; we write everything in between). This
//! file is the adapter between two shapes: `rustls` wants an object with `start`, `hash` and
//! `output_len`, and `sha2` offers `Digest`.

use alloc::boxed::Box;

use rustls::crypto::hash::{Context, Hash, HashAlgorithm, Output};
use sha2::Digest;

/// SHA-256, the hash of `TLS13_AES_128_GCM_SHA256` and `TLS13_CHACHA20_POLY1305_SHA256`.
pub static SHA256: Algorithm<sha2::Sha256> = Algorithm {
    algorithm: HashAlgorithm::SHA256,
    output_len: 32,
    _marker: core::marker::PhantomData,
};

/// SHA-384, the hash of `TLS13_AES_256_GCM_SHA384`.
pub static SHA384: Algorithm<sha2::Sha384> = Algorithm {
    algorithm: HashAlgorithm::SHA384,
    output_len: 48,
    _marker: core::marker::PhantomData,
};

/// One hash function, generic over which `sha2` type implements it.
///
/// The `output_len` is carried rather than asked of the type because `rustls` wants it as a plain
/// `usize` on a `&self` method, and a `const` on the generic parameter would make this a harder
/// thing to read for no gain.
// **`PhantomData<fn() -> T>` rather than `PhantomData<T>`, and it is a safety choice rather than a
// style one.** These types are `static`s, so they must be `Send` and `Sync`. With `PhantomData<T>`
// they inherit `T`'s auto traits and would need a hand-written `unsafe impl` asserting the
// compiler is wrong, which `script/lint`'s unsafe-obligation count is right to object to. A
// function pointer returning `T` carries the same type information to the reader and is
// unconditionally `Send + Sync`, because it owns nothing.
#[derive(Debug)]
pub struct Algorithm<D> {
    algorithm: HashAlgorithm,
    output_len: usize,
    _marker: core::marker::PhantomData<fn() -> D>,
}


impl<D> Hash for Algorithm<D>
where
    D: Digest + Clone + Send + Sync + 'static,
{
    fn start(&self) -> Box<dyn Context> {
        Box::new(Running::<D>(D::new()))
    }

    fn hash(&self, data: &[u8]) -> Output {
        Output::new(&D::digest(data)[..])
    }

    fn output_len(&self) -> usize {
        self.output_len
    }

    fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }
}

/// A hash computation in progress.
///
/// `rustls` needs both a consuming finish and a **forking** one, because TLS 1.3's transcript hash
/// is read at several points while more handshake messages are still to come. `Clone` on `sha2`'s
/// types is what makes the fork a copy of the state rather than a replay of the input.
struct Running<D>(D);

impl<D> Context for Running<D>
where
    D: Digest + Clone + Send + Sync + 'static,
{
    fn fork_finish(&self) -> Output {
        Output::new(&self.0.clone().finalize()[..])
    }

    fn fork(&self) -> Box<dyn Context> {
        Box::new(Running::<D>(self.0.clone()))
    }

    fn finish(self: Box<Self>) -> Output {
        Output::new(&self.0.finalize()[..])
    }

    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }
}
