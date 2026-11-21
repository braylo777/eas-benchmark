//! Bit-exact port of CPython's `random.Random` (MT19937) for the calls `probe_data` uses:
//! integer seeding, `getrandbits(k ≤ 32)`, `_randbelow`, `randint`, `choice`, `sample`.

const N: usize = 624;
const M: usize = 397;

pub struct PyRandom {
    mt: [u32; N],
    idx: usize,
}

impl PyRandom {
    /// Same as `random.Random(seed)` for a non-negative integer seed.
    pub fn new(seed: u64) -> Self {
        let mut key: Vec<u32> = Vec::new();
        let mut s = seed;
        loop {
            key.push((s & 0xffff_ffff) as u32);
            s >>= 32;
            if s == 0 {
                break;
            }
        }
        let mut r = PyRandom { mt: [0; N], idx: N };
        r.init_by_array(&key);
        r
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..N {
            let prev = self.mt[i - 1];
            self.mt[i] = 1_812_433_253u32
                .wrapping_mul(prev ^ (prev >> 30))
                .wrapping_add(i as u32);
        }
        self.idx = N;
    }

    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19_650_218);
        let (mut i, mut j) = (1usize, 0usize);
        let mut k = N.max(key.len());
        while k > 0 {
            let prev = self.mt[i - 1];
            self.mt[i] = (self.mt[i] ^ (prev ^ (prev >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
            k -= 1;
        }
        k = N - 1;
        while k > 0 {
            let prev = self.mt[i - 1];
            self.mt[i] = (self.mt[i] ^ (prev ^ (prev >> 30)).wrapping_mul(1_566_083_941))
                .wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            k -= 1;
        }
        self.mt[0] = 0x8000_0000;
    }

    /// One tempered 32-bit output (`genrand_uint32`).
    pub fn genrand_u32(&mut self) -> u32 {
        if self.idx >= N {
            for kk in 0..N {
                let y = (self.mt[kk] & 0x8000_0000) | (self.mt[(kk + 1) % N] & 0x7fff_ffff);
                let mag = if y & 1 == 1 { 0x9908_b0df } else { 0 };
                self.mt[kk] = self.mt[(kk + M) % N] ^ (y >> 1) ^ mag;
            }
            self.idx = 0;
        }
        let mut y = self.mt[self.idx];
        self.idx += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// `getrandbits(k)` for 0 ≤ k ≤ 32.
    pub fn getrandbits(&mut self, k: u32) -> u32 {
        assert!(k <= 32, "getrandbits only ported for k <= 32");
        if k == 0 {
            return 0;
        }
        self.genrand_u32() >> (32 - k)
    }

    /// `_randbelow_with_getrandbits(n)`, 1 ≤ n < 2^32.
    pub fn randbelow(&mut self, n: u64) -> u64 {
        assert!(n >= 1 && n <= u32::MAX as u64, "randbelow range");
        let k = 64 - n.leading_zeros();
        let mut r = self.getrandbits(k) as u64;
        while r >= n {
            r = self.getrandbits(k) as u64;
        }
        r
    }

    /// `randint(a, b)`, inclusive.
    pub fn randint(&mut self, a: i64, b: i64) -> i64 {
        assert!(b >= a, "empty randint range");
        a + self.randbelow((b - a + 1) as u64) as i64
    }

    /// `choice(seq)`.
    pub fn choice<T: Copy>(&mut self, seq: &[T]) -> T {
        seq[self.randbelow(seq.len() as u64) as usize]
    }

    /// `sample(population, k)`, both CPython branches (pool and set).
    pub fn sample<T: Copy>(&mut self, pop: &[T], k: usize) -> Vec<T> {
        let n = pop.len();
        assert!(k <= n, "sample larger than population");
        let mut setsize = 21usize;
        if k > 5 {
            let e = ((k * 3) as f64).log(4.0).ceil() as u32;
            setsize += 4usize.pow(e);
        }
        let mut out = Vec::with_capacity(k);
        if n <= setsize {
            let mut pool = pop.to_vec();
            for i in 0..k {
                let j = self.randbelow((n - i) as u64) as usize;
                out.push(pool[j]);
                pool[j] = pool[n - i - 1];
            }
        } else {
            let mut sel = std::collections::HashSet::new();
            for _ in 0..k {
                let mut j = self.randbelow(n as u64) as usize;
                while sel.contains(&j) {
                    j = self.randbelow(n as u64) as usize;
                }
                sel.insert(j);
                out.push(pop[j]);
            }
        }
        out
    }
}
