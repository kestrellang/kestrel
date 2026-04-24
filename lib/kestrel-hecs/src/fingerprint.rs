use std::hash::{Hash, Hasher};

/// 128-bit fingerprint for change detection.
///
/// Computed by hashing a value with two different seeds. When a query
/// re-executes and produces the same fingerprint, downstream dependents
/// can skip re-execution (early cutoff / backdating).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Fingerprint {
    lo: u64,
    hi: u64,
}

impl Fingerprint {
    pub const ZERO: Self = Self { lo: 0, hi: 0 };

    /// Compute a fingerprint for any hashable value.
    pub fn of<T: Hash>(value: &T) -> Self {
        // Two independent hashes from different initial states
        // give us 128 bits of collision resistance.
        let lo = {
            let mut h = SipHasher::new_with_keys(0, 0);
            value.hash(&mut h);
            h.finish()
        };
        let hi = {
            let mut h = SipHasher::new_with_keys(0x517cc1b727220a95, 0x6c62272e07bb0142);
            value.hash(&mut h);
            h.finish()
        };
        Self { lo, hi }
    }
}

/// Minimal SipHash-2-4 hasher with configurable keys.
///
/// We roll our own here to avoid depending on `std::hash::DefaultHasher`
/// which doesn't guarantee key customization across Rust versions.
struct SipHasher {
    v0: u64,
    v1: u64,
    v2: u64,
    v3: u64,
    buf: [u8; 8],
    buf_len: usize,
    total_len: usize,
}

impl SipHasher {
    fn new_with_keys(key0: u64, key1: u64) -> Self {
        Self {
            v0: key0 ^ 0x736f6d6570736575,
            v1: key1 ^ 0x646f72616e646f6d,
            v2: key0 ^ 0x6c7967656e657261,
            v3: key1 ^ 0x7465646279746573,
            buf: [0; 8],
            buf_len: 0,
            total_len: 0,
        }
    }

    fn sip_round(&mut self) {
        self.v0 = self.v0.wrapping_add(self.v1);
        self.v1 = self.v1.rotate_left(13);
        self.v1 ^= self.v0;
        self.v0 = self.v0.rotate_left(32);
        self.v2 = self.v2.wrapping_add(self.v3);
        self.v3 = self.v3.rotate_left(16);
        self.v3 ^= self.v2;
        self.v0 = self.v0.wrapping_add(self.v3);
        self.v3 = self.v3.rotate_left(21);
        self.v3 ^= self.v0;
        self.v2 = self.v2.wrapping_add(self.v1);
        self.v1 = self.v1.rotate_left(17);
        self.v1 ^= self.v2;
        self.v2 = self.v2.rotate_left(32);
    }

    fn process_block(&mut self, m: u64) {
        self.v3 ^= m;
        self.sip_round();
        self.sip_round();
        self.v0 ^= m;
    }
}

impl Hasher for SipHasher {
    fn write(&mut self, msg: &[u8]) {
        self.total_len += msg.len();
        let mut offset = 0;

        // Fill the buffer if partially full
        if self.buf_len > 0 {
            let need = 8 - self.buf_len;
            let take = need.min(msg.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&msg[..take]);
            self.buf_len += take;
            offset = take;
            if self.buf_len == 8 {
                let m = u64::from_le_bytes(self.buf);
                self.process_block(m);
                self.buf_len = 0;
            }
        }

        // Process full 8-byte blocks
        while offset + 8 <= msg.len() {
            let m = u64::from_le_bytes(msg[offset..offset + 8].try_into().unwrap());
            self.process_block(m);
            offset += 8;
        }

        // Buffer remaining bytes
        let remaining = msg.len() - offset;
        if remaining > 0 {
            self.buf[..remaining].copy_from_slice(&msg[offset..]);
            self.buf_len = remaining;
        }
    }

    fn finish(&self) -> u64 {
        // Finalize: pad the last block with total length
        let mut v0 = self.v0;
        let mut v1 = self.v1;
        let mut v2 = self.v2;
        let mut v3 = self.v3;

        let mut last = (self.total_len as u64 & 0xff) << 56;
        let buf = &self.buf[..self.buf_len];
        for (i, &b) in buf.iter().enumerate() {
            last |= (b as u64) << (i * 8);
        }

        v3 ^= last;
        // Two c rounds
        v0 = v0.wrapping_add(v1);
        v1 = v1.rotate_left(13);
        v1 ^= v0;
        v0 = v0.rotate_left(32);
        v2 = v2.wrapping_add(v3);
        v3 = v3.rotate_left(16);
        v3 ^= v2;
        v0 = v0.wrapping_add(v3);
        v3 = v3.rotate_left(21);
        v3 ^= v0;
        v2 = v2.wrapping_add(v1);
        v1 = v1.rotate_left(17);
        v1 ^= v2;
        v2 = v2.rotate_left(32);
        // Second round
        v0 = v0.wrapping_add(v1);
        v1 = v1.rotate_left(13);
        v1 ^= v0;
        v0 = v0.rotate_left(32);
        v2 = v2.wrapping_add(v3);
        v3 = v3.rotate_left(16);
        v3 ^= v2;
        v0 = v0.wrapping_add(v3);
        v3 = v3.rotate_left(21);
        v3 ^= v0;
        v2 = v2.wrapping_add(v1);
        v1 = v1.rotate_left(17);
        v1 ^= v2;
        v2 = v2.rotate_left(32);

        v0 ^= last;
        v2 ^= 0xff;

        // Four d rounds
        for _ in 0..4 {
            v0 = v0.wrapping_add(v1);
            v1 = v1.rotate_left(13);
            v1 ^= v0;
            v0 = v0.rotate_left(32);
            v2 = v2.wrapping_add(v3);
            v3 = v3.rotate_left(16);
            v3 ^= v2;
            v0 = v0.wrapping_add(v3);
            v3 = v3.rotate_left(21);
            v3 ^= v0;
            v2 = v2.wrapping_add(v1);
            v1 = v1.rotate_left(17);
            v1 ^= v2;
            v2 = v2.rotate_left(32);
        }

        v0 ^ v1 ^ v2 ^ v3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_value_same_fingerprint() {
        let a = Fingerprint::of(&"hello world");
        let b = Fingerprint::of(&"hello world");
        assert_eq!(a, b);
    }

    #[test]
    fn different_values_different_fingerprints() {
        let a = Fingerprint::of(&"hello");
        let b = Fingerprint::of(&"world");
        assert_ne!(a, b);
    }

    #[test]
    fn lo_and_hi_are_independent() {
        let fp = Fingerprint::of(&42u64);
        // The two halves should be different (different seeds)
        assert_ne!(fp.lo, fp.hi);
    }

    #[test]
    fn fingerprint_zero() {
        let zero = Fingerprint::ZERO;
        assert_eq!(zero.lo, 0);
        assert_eq!(zero.hi, 0);
        // Any real fingerprint should differ from zero
        let fp = Fingerprint::of(&"anything");
        assert_ne!(fp, Fingerprint::ZERO);
    }

    #[test]
    fn struct_fingerprint() {
        #[derive(Hash)]
        struct Foo {
            x: i32,
            y: String,
        }
        let a = Fingerprint::of(&Foo {
            x: 1,
            y: "hi".into(),
        });
        let b = Fingerprint::of(&Foo {
            x: 1,
            y: "hi".into(),
        });
        let c = Fingerprint::of(&Foo {
            x: 2,
            y: "hi".into(),
        });
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
