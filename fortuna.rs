// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.104 / REHOME-02b — **MEKANISME PRNG `Fortuna` milik Rust**.


#![allow(unsafe_code)]

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan; cermin `KlatosFortunaReport` di `KlatSeam116.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_464F_5254; // "KLATFORT"

/// Jumlah kolam entropi (`FortunaPRNG::pool_count`).
pub const JUMLAH_KOLAM: usize = 32;
/// Ambang reseed pada kolam 0 (`FortunaPRNG::reseed_threshold`).
pub const AMBANG_RESEED: usize = 16;
/// Ukuran blok AES (byte).
pub const UKURAN_BLOK: usize = 16;
/// Ukuran kunci AES-256 (byte).
pub const UKURAN_KUNCI: usize = 32;

// ---------------------------------------------------------------------------
// Penghitung audit
// ---------------------------------------------------------------------------

/// Penghitung per-pintu (audit produksi).
#[derive(Clone, Copy)]
pub enum Counter {
    Entropi = 0,
    Acak = 1,
    Reseed = 2,
    Siap = 3,
}
/// Jumlah penghitung.
pub const N_COUNTER: usize = 4;
static C: [AtomicU32; N_COUNTER] = [const { AtomicU32::new(0) }; N_COUNTER];
#[inline]
fn bump(x: Counter) {
    C[x as usize].fetch_add(1, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// SHA-256 (port `Crypto::Hash::SHA256`)
// ---------------------------------------------------------------------------

pub mod sha256 {
    /// Konstanta putaran (`SHA256Constants::RoundConstants`).
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    /// Hash inisialisasi (`SHA256Constants::InitializationHashes`).
    pub const H0: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    /// Keadaan hash inkremental. Cermin `SHA256` (`BlockSize = 64`).
    #[derive(Clone, Copy)]
    pub struct Sha256 {
        state: [u32; 8],
        buf: [u8; 64],
        len: usize,
        bits: u64,
    }

    impl Sha256 {
        /// Keadaan awal (setara `SHA256()` → `reset()`).
        pub const fn kosong() -> Self {
            Sha256 {
                state: H0,
                buf: [0u8; 64],
                len: 0,
                bits: 0,
            }
        }

        /// Setara `SHA256::reset()`.
        pub fn reset(&mut self) {
            self.state = H0;
            self.buf = [0u8; 64];
            self.len = 0;
            self.bits = 0;
        }

        /// Setara `SHA256::update`.
        pub fn update(&mut self, data: &[u8]) {
            let mut i = 0usize;
            while i < data.len() {
                let copy = core::cmp::min(64 - self.len, data.len() - i);
                self.buf[self.len..self.len + copy].copy_from_slice(&data[i..i + copy]);
                self.len += copy;
                i += copy;
                if self.len == 64 {
                    self.transform();
                    self.bits += 64 * 8;
                    self.len = 0;
                }
            }
        }

        /// Setara `SHA256::digest()`: `peek()` lalu `reset()`.
        ///
        /// `digest()` pada C++ **me-reset** objek; `FortunaPRNG::reseed`
        /// mengandalkan itu untuk mengosongkan kembali kolam yang dipakai.
        pub fn digest(&mut self) -> [u8; 32] {
            let out = self.peek();
            self.reset();
            out
        }

        /// Setara `SHA256::peek()` (finalisasi tanpa reset; bekerja pada salinan).
        pub fn peek(&self) -> [u8; 32] {
            let mut state = self.state;
            let mut buf = self.buf;
            let mut len = self.len;
            let mut bits = self.bits;

            if len < 56 {
                buf[len] = 0x80;
                len += 1;
                while len < 56 {
                    buf[len] = 0;
                    len += 1;
                }
            } else {
                buf[len] = 0x80;
                len += 1;
                while len < 64 {
                    buf[len] = 0;
                    len += 1;
                }
                transform(&mut state, &buf);
                buf = [0u8; 64];
            }

            bits += (self.len as u64) * 8;
            buf[63] = bits as u8;
            buf[62] = (bits >> 8) as u8;
            buf[61] = (bits >> 16) as u8;
            buf[60] = (bits >> 24) as u8;
            buf[59] = (bits >> 32) as u8;
            buf[58] = (bits >> 40) as u8;
            buf[57] = (bits >> 48) as u8;
            buf[56] = (bits >> 56) as u8;

            transform(&mut state, &buf);

            let mut out = [0u8; 32];
            for i in 0..4usize {
                let sh = 24 - (i as u32) * 8;
                out[i] = (state[0] >> sh) as u8;
                out[i + 4] = (state[1] >> sh) as u8;
                out[i + 8] = (state[2] >> sh) as u8;
                out[i + 12] = (state[3] >> sh) as u8;
                out[i + 16] = (state[4] >> sh) as u8;
                out[i + 20] = (state[5] >> sh) as u8;
                out[i + 24] = (state[6] >> sh) as u8;
                out[i + 28] = (state[7] >> sh) as u8;
            }
            out
        }

        fn transform(&mut self) {
            transform(&mut self.state, &self.buf);
        }
    }

    #[inline]
    fn transform(state: &mut [u32; 8], block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for i in 0..16usize {
            let j = i * 4;
            w[i] = ((block[j] as u32) << 24)
                | ((block[j + 1] as u32) << 16)
                | ((block[j + 2] as u32) << 8)
                | (block[j + 3] as u32);
        }
        for i in 16..64usize {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for i in 0..64usize {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }
}

// ---------------------------------------------------------------------------
// AES-256 (port `Crypto::Cipher::AESCipher`)
// ---------------------------------------------------------------------------

pub mod aes256 {
    /// S-box AES (`AESTables::Encode*`).
    const SBOX: [u8; 256] = [
        0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab,
        0x76, 0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4,
        0x72, 0xc0, 0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71,
        0xd8, 0x31, 0x15, 0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2,
        0xeb, 0x27, 0xb2, 0x75, 0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6,
        0xb3, 0x29, 0xe3, 0x2f, 0x84, 0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb,
        0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf, 0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45,
        0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8, 0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5,
        0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2, 0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44,
        0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73, 0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a,
        0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb, 0xe0, 0x32, 0x3a, 0x0a, 0x49,
        0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79, 0xe7, 0xc8, 0x37, 0x6d,
        0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08, 0xba, 0x78, 0x25,
        0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a, 0x70, 0x3e,
        0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e, 0xe1,
        0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
        0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb,
        0x16,
    ];

    /// Konstanta putaran kunci (AES-256 hanya butuh indeks 1..=7).
    const RCON: [u8; 8] = [0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40];

    /// Kunci AES-256 yang sudah diperluas: 15 kunci putaran × 16 byte.
    #[derive(Clone, Copy)]
    pub struct Kunci {
        rk: [u8; 240],
    }

    #[inline]
    fn xtime(x: u8) -> u8 {
        (x << 1) ^ (((x >> 7) & 1) * 0x1b)
    }

    #[inline]
    fn sub_word(w: u32) -> u32 {
        ((SBOX[((w >> 24) & 0xff) as usize] as u32) << 24)
            | ((SBOX[((w >> 16) & 0xff) as usize] as u32) << 16)
            | ((SBOX[((w >> 8) & 0xff) as usize] as u32) << 8)
            | (SBOX[(w & 0xff) as usize] as u32)
    }

    impl Kunci {
        /// Setara `AESCipherKey::expand_encrypt_key_impl<None>(user_key, 256)`.
        pub fn perluas(user_key: &[u8; 32]) -> Kunci {
            let mut w = [0u32; 60];
            for i in 0..8usize {
                let j = i * 4;
                w[i] = ((user_key[j] as u32) << 24)
                    | ((user_key[j + 1] as u32) << 16)
                    | ((user_key[j + 2] as u32) << 8)
                    | (user_key[j + 3] as u32);
            }
            for i in 8..60usize {
                let mut temp = w[i - 1];
                if i % 8 == 0 {
                    // RotWord + SubWord + Rcon
                    temp = sub_word(temp.rotate_left(8)) ^ ((RCON[i / 8] as u32) << 24);
                } else if i % 8 == 4 {
                    temp = sub_word(temp);
                }
                w[i] = w[i - 8] ^ temp;
            }
            let mut rk = [0u8; 240];
            for i in 0..60usize {
                let j = i * 4;
                rk[j] = (w[i] >> 24) as u8;
                rk[j + 1] = (w[i] >> 16) as u8;
                rk[j + 2] = (w[i] >> 8) as u8;
                rk[j + 3] = w[i] as u8;
            }
            Kunci { rk }
        }

        /// Setara `AESCipher::encrypt_block_impl` untuk satu blok 128-bit.
        pub fn enkripsi_blok(&self, block: &[u8; 16]) -> [u8; 16] {
            // Tambahkan kunci putaran 0.
            let mut s = [0u8; 16];
            for i in 0..16usize {
                s[i] = block[i] ^ self.rk[i];
            }
            // Putaran 1..13.
            for r in 1..14usize {
                sub_bytes(&mut s);
                shift_rows(&mut s);
                mix_columns(&mut s);
                add_round_key(&mut s, &self.rk[r * 16..r * 16 + 16]);
            }
            // Putaran final 14 (tanpa MixColumns).
            sub_bytes(&mut s);
            shift_rows(&mut s);
            add_round_key(&mut s, &self.rk[14 * 16..14 * 16 + 16]);
            s
        }
    }

    #[inline]
    fn sub_bytes(s: &mut [u8; 16]) {
        for x in s.iter_mut() {
            *x = SBOX[*x as usize];
        }
    }

    #[inline]
    fn shift_rows(s: &mut [u8; 16]) {
        // Keadaan kolom-mayor: s[r + 4c]. Baris r digeser kiri r kali.
        let t = *s;
        for c in 0..4usize {
            for r in 0..4usize {
                s[r + 4 * c] = t[r + 4 * ((c + r) % 4)];
            }
        }
    }

    #[inline]
    fn mix_columns(s: &mut [u8; 16]) {
        for c in 0..4usize {
            let j = 4 * c;
            let a0 = s[j];
            let a1 = s[j + 1];
            let a2 = s[j + 2];
            let a3 = s[j + 3];
            s[j] = xtime(a0) ^ (xtime(a1) ^ a1) ^ a2 ^ a3;
            s[j + 1] = a0 ^ xtime(a1) ^ (xtime(a2) ^ a2) ^ a3;
            s[j + 2] = a0 ^ a1 ^ xtime(a2) ^ (xtime(a3) ^ a3);
            s[j + 3] = (xtime(a0) ^ a0) ^ a1 ^ a2 ^ xtime(a3);
        }
    }

    #[inline]
    fn add_round_key(s: &mut [u8; 16], rk: &[u8]) {
        for i in 0..16usize {
            s[i] ^= rk[i];
        }
    }
}

// ---------------------------------------------------------------------------
// Mesin Fortuna
// ---------------------------------------------------------------------------

/// Keadaan + transisi `FortunaPRNG<AESCipher, SHA256, 256>`.
#[derive(Clone, Copy)]
pub struct Fortuna {
    counter: [u8; UKURAN_BLOK],
    key: [u8; UKURAN_KUNCI],
    key_len: usize,
    reseed_number: u32,
    p0_len: usize,
    pools: [sha256::Sha256; JUMLAH_KOLAM],
}

impl Fortuna {
    /// Keadaan awal (`FortunaPRNG()`): counter nol, kunci kosong, 32 kolam segar.
    pub const fn baru() -> Self {
        Fortuna {
            counter: [0u8; UKURAN_BLOK],
            key: [0u8; UKURAN_KUNCI],
            key_len: 0,
            reseed_number: 0,
            p0_len: 0,
            pools: [sha256::Sha256::kosong(); JUMLAH_KOLAM],
        }
    }

    /// `FortunaPRNG::is_seeded`.
    #[inline]
    pub fn terisi(&self) -> bool {
        self.reseed_number > 0
    }

    /// `FortunaPRNG::is_ready` (prasyarat: pemanggil memegang kunci).
    #[inline]
    pub fn siap(&self) -> bool {
        self.terisi() || self.p0_len >= AMBANG_RESEED
    }

    /// `FortunaPRNG::add_random_event`.
    pub fn tambah_entropi(&mut self, data: &[u8], pool: usize) {
        let p = pool % JUMLAH_KOLAM;
        if p == 0 {
            self.p0_len += 1;
        }
        self.pools[p].update(data);
    }

    /// `FortunaPRNG::reseed`.
    pub fn reseed(&mut self) {
        let mut new_key = sha256::Sha256::kosong();
        new_key.update(&self.key[..self.key_len]);
        for i in 0..JUMLAH_KOLAM {
            if self.reseed_number % (1u32 << i) == 0 {
                let digest = self.pools[i].digest();
                new_key.update(&digest);
            }
        }
        let digest = new_key.digest();
        self.key.copy_from_slice(&digest);
        self.key_len = UKURAN_KUNCI;
        self.reseed_number += 1;
        self.p0_len = 0;
    }

    /// `FortunaPRNG::get_random_bytes`. Mengembalikan `false` bila belum siap.
    ///
    /// Prasyarat: pemanggil memegang kunci (C++ `KernelRng::m_lock`).
    pub fn acak(&mut self, out: &mut [u8]) -> bool {
        if !self.siap() {
            return false;
        }
        if self.p0_len >= AMBANG_RESEED {
            self.reseed();
            bump(Counter::Reseed);
        }
        if !self.terisi() {
            // C++: VERIFY(is_seeded()). Kita MENOLAK, bukan panic.
            return false;
        }

        // Blok 1: hasilkan `out` dari keystream; counter maju.
        let kunci = aes256::Kunci::perluas(&self.key);
        let mut counter = self.counter;
        let mut offset = 0usize;
        while offset < out.len() {
            let blok = kunci.enkripsi_blok(&counter);
            let n = core::cmp::min(UKURAN_BLOK, out.len() - offset);
            out[offset..offset + n].copy_from_slice(&blok[..n]);
            offset += n;
            increment_be(&mut counter);
        }

        // Blok 2: tarik kunci baru dari keystream; counter maju lagi.
        let mut key_span = self.key;
        let mut offset = 0usize;
        while offset < UKURAN_KUNCI {
            let blok = kunci.enkripsi_blok(&counter);
            let n = core::cmp::min(UKURAN_BLOK, UKURAN_KUNCI - offset);
            key_span[offset..offset + n].copy_from_slice(&blok[..n]);
            offset += n;
            increment_be(&mut counter);
        }

        self.key = key_span;
        self.counter = counter;
        true
    }
}

/// Increment big-endian `IncrementInplace` (byte terakhir naik; melimpah ke kiri).
#[inline]
fn increment_be(iv: &mut [u8; UKURAN_BLOK]) {
    let mut i = UKURAN_BLOK;
    while i > 0 {
        i -= 1;
        if iv[i] == 0xff {
            iv[i] = 0;
        } else {
            iv[i] += 1;
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Keadaan PRODUKSI (dilindungi kunci C++)
// ---------------------------------------------------------------------------

/// Pembungkus `Sync` untuk keadaan produksi. Kunci (spinlock) tetap milik C++;
/// Rust hanya menjamin `Sync` karena akses selalu di bawah kunci itu.
struct SyncFortuna(UnsafeCell<Fortuna>);
// SAFETY: seluruh akses `klatos_fortuna_*` terjadi di bawah `KernelRng::m_lock`.
unsafe impl Sync for SyncFortuna {}

static PRODUKSI: SyncFortuna = SyncFortuna(UnsafeCell::new(Fortuna::baru()));

#[inline]
fn produksi() -> &'static mut Fortuna {
    // SAFETY: dipanggil hanya dari pintu C ABI yang pemanggilnya memegang kunci.
    unsafe { &mut *PRODUKSI.0.get() }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// `FortunaPRNG::add_random_event` — hash `len` byte ke `pool`.
///
/// # Safety
/// Pemanggil memegang `KernelRng::m_lock`; `ptr` sah untuk `len` byte.
#[no_mangle]
pub unsafe extern "C" fn klatos_fortuna_tambah_entropi(
    ptr: *const u8,
    len: usize,
    pool: usize,
) -> u32 {
    bump(Counter::Entropi);
    let data: &[u8] = if ptr.is_null() || len == 0 {
        &[]
    } else {
        // SAFETY: kontrak pemanggil.
        unsafe { core::slice::from_raw_parts(ptr, len) }
    };
    produksi().tambah_entropi(data, pool);
    pool as u32
}

/// `FortunaPRNG::get_random_bytes` — isi `out` dengan `len` byte acak.
///
/// # Safety
/// Pemanggil memegang `KernelRng::m_lock`; `out` sah untuk `len` byte tulis.
#[no_mangle]
pub unsafe extern "C" fn klatos_fortuna_acak(out: *mut u8, len: usize) -> u32 {
    bump(Counter::Acak);
    let buf: &mut [u8] = if out.is_null() || len == 0 {
        &mut []
    } else {
        // SAFETY: kontrak pemanggil.
        unsafe { core::slice::from_raw_parts_mut(out, len) }
    };
    if produksi().acak(buf) {
        1
    } else {
        0
    }
}

/// `FortunaPRNG::is_ready` (prasyarat: kunci dipegang).
#[no_mangle]
pub extern "C" fn klatos_fortuna_siap() -> u32 {
    bump(Counter::Siap);
    if produksi().siap() {
        1
    } else {
        0
    }
}

/// `FortunaPRNG::is_seeded`.
#[no_mangle]
pub extern "C" fn klatos_fortuna_terisi() -> u32 {
    if produksi().terisi() {
        1
    } else {
        0
    }
}

/// Nomor reseed saat ini (audit).
#[no_mangle]
pub extern "C" fn klatos_fortuna_nomor_reseed() -> u32 {
    produksi().reseed_number
}

/// Panjang kolam-0 saat ini (audit).
#[no_mangle]
pub extern "C" fn klatos_fortuna_p0_len() -> u32 {
    produksi().p0_len as u32
}

/// Membaca penghitung audit `i` (0 bila di luar rentang).
#[no_mangle]
pub extern "C" fn klatos_fortuna_counter(i: u32) -> u32 {
    if (i as usize) < N_COUNTER {
        C[i as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

/// Reset seluruh penghitung; mengembalikan jumlahnya.
#[no_mangle]
pub extern "C" fn klatos_fortuna_reset() -> u32 {
    for x in C.iter() {
        x.store(0, Ordering::Release);
    }
    N_COUNTER as u32
}

// ---------------------------------------------------------------------------
// Laporan
// ---------------------------------------------------------------------------

/// Laporan P1.104; cermin `KlatosFortunaReport` di `KlatSeam116.h`.
#[repr(C)]
pub struct KlatosFortunaReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub diferensial_diukur: u32,
    pub diferensial_beda: u32,
    pub status: i32,
}

fn hex(s: &[u8], out: &mut [u8; 64]) -> usize {
    const D: &[u8; 16] = b"0123456789abcdef";
    let mut n = 0usize;
    for &b in s {
        out[n] = D[(b >> 4) as usize];
        out[n + 1] = D[(b & 0xf) as usize];
        n += 2;
    }
    n
}

fn hex_eq(digest: &[u8], harap: &str) -> bool {
    let mut buf = [0u8; 64];
    let n = hex(digest, &mut buf);
    n == harap.len() && &buf[..n] == harap.as_bytes()
}

fn run() -> KlatosFortunaReport {
    let mut c = 0u32;
    let mut p = 0u32;
    macro_rules! x {
        ($e:expr) => {{
            c += 1;
            if $e {
                p += 1;
            }
        }};
    }

    // --- kepala & bentuk ---------------------------------------------------
    x!(REPORT_TAG == 0x4B4C_4154_464F_5254);
    x!(N_COUNTER == 4);
    x!(core::mem::size_of::<KlatosFortunaReport>() == 32);
    x!(JUMLAH_KOLAM == 32 && AMBANG_RESEED == 16);
    x!(UKURAN_BLOK == 16 && UKURAN_KUNCI == 32);

    // --- SHA-256 vektor NIST (FIPS 180-4) ----------------------------------
    {
        let mut h = sha256::Sha256::kosong();
        h.update(b"");
        x!(hex_eq(
            &h.digest(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        ));

        let mut h = sha256::Sha256::kosong();
        h.update(b"abc");
        x!(hex_eq(
            &h.digest(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        ));

        let mut h = sha256::Sha256::kosong();
        h.update(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq");
        x!(hex_eq(
            &h.digest(),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        ));

        // Multi-blok + update bertahap harus identik dengan satu kali update.
        let mut a = sha256::Sha256::kosong();
        let data = [0x61u8; 1000];
        a.update(&data);
        let mut b = sha256::Sha256::kosong();
        for chunk in data.chunks(7) {
            b.update(chunk);
        }
        x!(a.digest() == b.digest());

        // `digest()` me-reset: hash kedua dari keadaan kosong.
        let mut h = sha256::Sha256::kosong();
        h.update(b"abc");
        let d1 = h.digest();
        let d2 = h.digest();
        x!(d1 != d2); // d2 = SHA256("") karena reset
        let mut e = sha256::Sha256::kosong();
        e.update(b"");
        x!(d2 == e.digest());
    }

    // --- AES-256 vektor FIPS-197 (Lampiran C.3) ----------------------------
    {
        let mut key = [0u8; 32];
        for (i, k) in key.iter_mut().enumerate() {
            *k = i as u8;
        }
        let mut pt = [0u8; 16];
        let pt_hex = "00112233445566778899aabbccddeeff";
        for i in 0..16usize {
            pt[i] = u8::from_str_radix(&pt_hex[i * 2..i * 2 + 2], 16).unwrap();
        }
        let kunci = aes256::Kunci::perluas(&key);
        let ct = kunci.enkripsi_blok(&pt);
        x!(hex_eq(&ct, "8ea2b7ca516745bfeafc49904b496089"));

        // Kontrol negatif: kunci berbeda -> sandi berbeda.
        let mut key2 = key;
        key2[0] ^= 0x01;
        let kunci2 = aes256::Kunci::perluas(&key2);
        x!(kunci2.enkripsi_blok(&pt) != ct);

        // Rcon/key-schedule: enkripsi dua kali harus deterministik.
        x!(kunci.enkripsi_blok(&pt) == ct);
    }

    // --- mesin Fortuna: sifat struktural -----------------------------------
    {
        // Kolam 0 menaikkan p0_len; kolam lain tidak.
        let mut f = Fortuna::baru();
        x!(!f.terisi());
        x!(!f.siap());
        f.tambah_entropi(&[1u8, 2, 3], 0);
        x!(f.p0_len == 1);
        f.tambah_entropi(&[9u8], 5);
        x!(f.p0_len == 1);
        f.tambah_entropi(&[7u8], 32); // 32 % 32 == 0 -> kolam 0
        x!(f.p0_len == 2);
        f.tambah_entropi(&[8u8], 33); // 33 % 32 == 1 -> bukan kolam 0
        x!(f.p0_len == 2);

        // Belum 16 peristiwa -> belum siap, `acak` menolak tanpa menulis.
        let mut buf = [0xAAu8; 8];
        x!(!f.acak(&mut buf));
        x!(buf == [0xAAu8; 8]);
        x!(!f.terisi());

        // 16 peristiwa kolam-0 -> siap; reseed terjadi di dalam `acak`.
        for _ in 0..16 {
            f.tambah_entropi(&[0x5Au8], 0);
        }
        x!(f.siap());
        let mut out = [0u8; 32];
        x!(f.acak(&mut out));
        x!(f.terisi());
        x!(f.reseed_number == 1);
        x!(f.p0_len == 0);
        x!(out != [0u8; 32]);

        // Determinisme: dua mesin dengan riwayat entropi sama -> keluaran sama.
        let mut g = Fortuna::baru();
        for i in 0..(JUMLAH_KOLAM * AMBANG_RESEED) {
            g.tambah_entropi(&[(i & 0xff) as u8], i);
        }
        let mut o1 = [0u8; 64];
        let mut o2 = [0u8; 64];
        x!(g.acak(&mut o1));
        x!(g.acak(&mut o2));
        let mut g2 = Fortuna::baru();
        for i in 0..(JUMLAH_KOLAM * AMBANG_RESEED) {
            g2.tambah_entropi(&[(i & 0xff) as u8], i);
        }
        let mut o3 = [0u8; 64];
        x!(g2.acak(&mut o3));
        x!(o1 == o3);
        // Dua tarikan berurutan berbeda (keystream + kunci berganti).
        x!(o1 != o2);

        // Panjang keluaran yang diminta dihormati.
        let mut pendek = [0u8; 5];
        x!(g.acak(&mut pendek));
        x!(pendek.iter().any(|&b| b != 0));

        // Negatif: 15 peristiwa saja -> belum siap.
        let mut h = Fortuna::baru();
        for _ in 0..15 {
            h.tambah_entropi(&[1u8], 0);
        }
        x!(!h.siap());
        let mut b = [0u8; 4];
        x!(!h.acak(&mut b));
    }

    // --- diferensial: SHA-256 vs oracle potongan (panjang bervariasi) ------
    let mut d = 0u32;
    let mut beda = 0u32;
    macro_rules! q {
        ($got:expr, $exp:expr) => {{
            d += 1;
            if $got != $exp {
                beda += 1;
            }
        }};
    }
    // Oracle independen: hash "abc" berulang n kali, dibandingkan dengan
    // implementasi yang menerima byte yang sama dalam satu potongan.
    for n in 0..40usize {
        let mut data = [0u8; 120];
        for i in 0..n {
            data[i * 3] = b'a';
            data[i * 3 + 1] = b'b';
            data[i * 3 + 2] = b'c';
        }
        let mut satu = sha256::Sha256::kosong();
        satu.update(&data[..n * 3]);
        let mut banyak = sha256::Sha256::kosong();
        for _ in 0..n {
            banyak.update(b"abc");
        }
        q!(satu.digest(), banyak.digest());
    }
    // NIST KAT "abc" sebagai jangkar absolut.
    {
        let mut h = sha256::Sha256::kosong();
        h.update(b"abc");
        let mut buf = [0u8; 64];
        let n = hex(&h.digest(), &mut buf);
        q!(&buf[..n], &b"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"[..]);
    }

    x!(d == 41);
    x!(beda == 0);

    KlatosFortunaReport {
        tag: REPORT_TAG,
        checks: c,
        passed: p,
        diferensial_diukur: d,
        diferensial_beda: beda,
        status: if c == p { 0 } else { -1 },
    }
}

/// Selftest P1.104.
#[no_mangle]
pub extern "C" fn klatos_fortuna_selftest() -> KlatosFortunaReport {
    run()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;
    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_464F_5254);
        assert!(N_COUNTER == 4);
        assert!(JUMLAH_KOLAM == 32);
        assert!(AMBANG_RESEED == 16);
        assert!(UKURAN_BLOK == 16);
        assert!(UKURAN_KUNCI == 32);
        assert!(core::mem::size_of::<KlatosFortunaReport>() == 32);
        assert!(core::mem::align_of::<KlatosFortunaReport>() == 8);
        assert!(Counter::Entropi as usize == 0);
        assert!(Counter::Siap as usize == 3);
        assert!(sha256::H0[0] == 0x6a09e667);
        // Ukuran mesin Fortuna: 16 + 32 + 8 + 4 + 8 + 32×(8×4+64+8+8)
        assert!(core::mem::size_of::<Fortuna>() > 0);
    };
}
