// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! Batas KSM (P0.97.5).


use crate::abi2::{object_kind, right};
use crate::dispatch;

/// Penanda mekanisme inti kernel Rust. Hanya ada di sisi Rust; kernel C++
/// mencetaknya untuk membuktikan kode Rust benar-benar dieksekusi.
pub const MECHANISM_TAG: u64 = 0x4B4C_4154_4B52_4E4C; // "KLATKRNL"

/// Bit "ditemukan" pada jawaban [`operation_rights`].
pub const FOUND: u64 = 1u64 << 32;

/// Jumlah entri tabel dispatch yang benar-benar dikompilasi ke dalam kernel.
pub fn table_count() -> u32 {
    dispatch::DISPATCH_TABLE.len() as u32
}

/// Hak yang diwajibkan operasi `(kind, opcode)`.
///
/// `0` berarti operasi tidak ada di tabel. Selain itu bentuknya
/// `FOUND | hak_wajib`.
///
/// `const` supaya jawabannya dapat dibuktikan pada waktu kompilasi di
/// [`conformance`] **dan** dipakai di runtime — satu implementasi, bukan dua.
pub const fn operation_rights(kind: u16, opcode: u8) -> u64 {
    match dispatch::lookup(kind, opcode) {
        Some(entry) => FOUND | entry.required as u64,
        None => 0,
    }
}

/// Satu baris yang diharapkan: `(kind, opcode, hak_wajib)`.
pub type Expected = (u16, u8, u32);

/// Model operasi yang harus dijawab tabel. Dijadikan `const` supaya dapat
/// dibuktikan pada waktu kompilasi di [`conformance`] **dan** dipakai di
/// runtime — satu daftar, bukan dua.
pub const EXPECTED: &[Expected] = &[
    (object_kind::ENDPOINT, dispatch::op_endpoint::SEND, right::SEND),
    (
        object_kind::ENDPOINT,
        dispatch::op_endpoint::RECEIVE,
        right::RECEIVE,
    ),
    (
        object_kind::ENDPOINT,
        dispatch::op_endpoint::GIVE,
        right::TRANSFER,
    ),
    (
        object_kind::ENDPOINT,
        dispatch::op_endpoint::CLOSE,
        right::NONE,
    ),
    (object_kind::DEVICE, dispatch::op_device::CLAIM, right::CONTROL),
    (
        object_kind::DEVICE,
        dispatch::op_device::LIFECYCLE,
        right::CONTROL,
    ),
    (object_kind::DEVICE, dispatch::op_device::INFO, right::NONE),
    (object_kind::TASK, dispatch::op_task::INFO, right::READ),
    (
        object_kind::TASK,
        dispatch::op_task::SETPGID,
        right::CONTROL,
    ),
    (object_kind::MEMORY, dispatch::op_mem::INFO, right::READ),
    (object_kind::MEMORY, dispatch::op_mem::MODIFY, right::CONTROL),
    (
        object_kind::SERVICE,
        dispatch::op_service::INFO,
        right::READ,
    ),
    (
        object_kind::SERVICE,
        dispatch::op_service::INVOKE,
        right::CONTROL,
    ),
    (
        object_kind::FILE_REF,
        dispatch::op_fileref::INFO,
        right::READ,
    ),
    (
        object_kind::FILE_REF,
        dispatch::op_fileref::MODIFY,
        right::CONTROL,
    ),
];

/// Hasil probe batas KSM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KsmFoundation {
    /// Jumlah entri tabel dispatch.
    pub table_entries: u32,
    /// Penanda mekanisme.
    pub mechanism_tag: u64,
    /// Jumlah pemeriksaan yang dijalankan.
    pub checks: u32,
    /// Jumlah pemeriksaan yang lulus.
    pub checks_passed: u32,
}

impl KsmFoundation {
    /// Lulus bila tabelnya adalah tabel yang diharapkan dan seluruh
    /// pemeriksaan lulus.
    pub const fn ok(&self) -> bool {
        self.table_entries == dispatch::DISPATCH_ENTRY_COUNT_EXPECTED
            && self.checks == self.checks_passed
            && self.checks > 0
    }
}

// ---------------------------------------------------------------------------
// P0.98 — SATU-SATUNYA PINTU IZIN (P0.98.2)
// ---------------------------------------------------------------------------

/// Token izin. **Medan privat** — hanya modul ini yang dapat membuatnya.
///
/// Inilah yang membuat P0.98 tidak menjadi otoritas kedua:
///
/// * Rust **tidak menyimpan** hak siapa pun. Tidak ada
///   `RustMemoryCapabilityTable` (P0.98.2) — tidak ada tabel hak sama sekali.
/// * `hak_dimiliki` **wajib** datang dari KSM C++ yang nyata. Rust hanya
///   menghitung predikat "hak wajib ⊆ hak dimiliki" terhadap tabel statis
///   P0.74 yang **sama**, lalu mengembalikan token bukti.
/// * Token tidak dapat ditempa: medannya privat, jadi `memory.rs` dan
///   `scheduler.rs` hanya dapat **menerima** `Izin`, tidak pernah membuatnya.
/// * Token mengikat `(kind, opcode)` yang mengizinkannya, sehingga token untuk
///   satu operasi tidak dapat dipakai untuk operasi lain — termasuk lintas
///   otoritas (mis. token MEMORY dipakai untuk operasi TASK).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Izin {
    kind: u16,
    opcode: u8,
}

impl Izin {
    /// `true` bila token ini memang diterbitkan untuk `(kind, opcode)` itu.
    pub const fn untuk(&self, kind: u16, opcode: u8) -> bool {
        self.kind == kind && self.opcode == opcode
    }

    /// Kind yang diizinkan token ini.
    pub const fn kind(&self) -> u16 {
        self.kind
    }

    /// Opcode yang diizinkan token ini.
    pub const fn opcode(&self) -> u8 {
        self.opcode
    }
}

/// Alasan penolakan. Setiap negatif harus **gagal-tertutup** (P0.98.6):
/// tidak ada varian "tidak tahu" yang membiarkan operasi lanjut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ditolak {
    /// `kind` tidak ada di kontrak ABI v2.
    KindTidakDikenal,
    /// `(kind, opcode)` tidak ada di tabel dispatch.
    OperasiTidakDikenal,
    /// Hak yang dimiliki tidak sah untuk kind ini — inilah penolakan
    /// **lintas otoritas** (mis. hak `SEND` sebuah endpoint dipakai untuk
    /// operasi MEMORY).
    HakTidakSahUntukKind,
    /// Hak yang dimiliki tidak memuat hak yang diwajibkan operasi.
    HakKurang,
}

/// Keputusan otorisasi. Hanya `Diizinkan` yang membawa [`Izin`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keputusan {
    Diizinkan(Izin),
    Ditolak(Ditolak),
}

impl Keputusan {
    pub const fn ok(&self) -> bool {
        matches!(self, Keputusan::Diizinkan(_))
    }
}

/// **Satu-satunya** cara membuat [`Izin`].
///
/// `hak_dimiliki` adalah hak yang **dimiliki pemanggil** menurut KSM C++ yang
/// nyata — bukan nilai yang dikarang Rust. Fungsi ini tidak menyimpan apa pun,
/// tidak mengubah keadaan, dan tidak pernah menambah hak: ia hanya menjawab
/// "apakah operasi ini, menurut tabel yang sama, boleh dilakukan dengan hak
/// yang sudah dimiliki".
///
/// Urutan pemeriksaan sengaja tetap supaya alasannya deterministik:
/// kind → operasi → keabsahan hak untuk kind → kecukupan hak.
pub const fn periksa_operasi(kind: u16, opcode: u8, hak_dimiliki: u32) -> Keputusan {
    if !object_kind::is_contract_defined(kind) {
        return Keputusan::Ditolak(Ditolak::KindTidakDikenal);
    }
    let entry = match dispatch::lookup(kind, opcode) {
        Some(entry) => entry,
        None => return Keputusan::Ditolak(Ditolak::OperasiTidakDikenal),
    };
    if !crate::abi2::rights_valid_for_kind(kind, hak_dimiliki) {
        return Keputusan::Ditolak(Ditolak::HakTidakSahUntukKind);
    }
    if !crate::abi2::rights_contain(hak_dimiliki, entry.required) {
        return Keputusan::Ditolak(Ditolak::HakKurang);
    }
    Keputusan::Diizinkan(Izin { kind, opcode })
}

/// Membuka token dari keputusan, atau `None` bila ditolak.
///
/// Dipakai model memori/task supaya setiap operasi yang membutuhkan otoritas
/// **wajib** menyerahkan token — tidak ada jalan pintas.
pub const fn izin_dari(keputusan: Keputusan) -> Option<Izin> {
    match keputusan {
        Keputusan::Diizinkan(izin) => Some(izin),
        Keputusan::Ditolak(_) => None,
    }
}

/// Menjalankan probe batas KSM terhadap tabel yang tertanam di kernel.
pub fn probe() -> KsmFoundation {
    let mut checks = 0u32;
    let mut checks_passed = 0u32;

    for &(kind, opcode, expected_right) in EXPECTED {
        checks += 1;
        if operation_rights(kind, opcode) == FOUND | expected_right as u64 {
            checks_passed += 1;
        }
    }

    // Dua keadaan yang harus dibedakan: opcode tak dikenal vs kind tak dikenal.
    checks += 2;
    if operation_rights(object_kind::ENDPOINT, 200) == 0 {
        checks_passed += 1;
    }
    if operation_rights(object_kind::NONE, dispatch::op_endpoint::SEND) == 0 {
        checks_passed += 1;
    }

    KsmFoundation {
        table_entries: table_count(),
        mechanism_tag: MECHANISM_TAG,
        checks,
        checks_passed,
    }
}

/// Konformansi waktu-kompilasi: model operasi di atas harus sama dengan tabel
/// statis P0.74. Dibuktikan di sini, bukan hanya di test.
pub mod conformance {
    use super::*;

    const _: () = {
        assert!(MECHANISM_TAG == 0x4B4C_4154_4B52_4E4C);
        // FASE-5 menumbuhkan tabel 15 -> 58 (append-only). `EXPECTED` sengaja
        // tetap 15: ia adalah MODEL UJI P0.97.5 untuk 15 entri lama, bukan
        // cermin penuh tabel. Entri FASE-5 diuji di `KlatIpc/src/dispatch.rs`.
        assert!(dispatch::DISPATCH_TABLE.len() == 60);
        assert!(EXPECTED.len() == 15);

        // --- P0.98: pintu izin, dibuktikan pada waktu kompilasi ---------------
        //
        // Hak yang SAH untuk kind-nya. `right::ALL` (0x3FF) sengaja TIDAK
        // dipakai: ia memuat bit yang tidak bermakna untuk kind mana pun
        // (`rights_valid_for_kind` menolaknya), jadi memakainya akan menguji
        // jalur yang salah.
        const HAK_MEMORI: u32 = right::CONTROL | right::READ | right::WRITE | right::MAP;
        const HAK_TASK: u32 = right::CONTROL | right::READ | right::WRITE;

        // Positif: hak sah yang memuat hak wajib diizinkan.
        assert!(matches!(
            periksa_operasi(object_kind::MEMORY, dispatch::op_mem::MODIFY, HAK_MEMORI),
            Keputusan::Diizinkan(_)
        ));
        // Negatif: hak kurang (READ saja tidak memuat CONTROL).
        assert!(matches!(
            periksa_operasi(object_kind::MEMORY, dispatch::op_mem::MODIFY, right::READ),
            Keputusan::Ditolak(Ditolak::HakKurang)
        ));
        // Negatif: kind tak dikenal.
        assert!(matches!(
            periksa_operasi(999, 0, HAK_MEMORI),
            Keputusan::Ditolak(Ditolak::KindTidakDikenal)
        ));
        // Negatif: operasi tak dikenal.
        assert!(matches!(
            periksa_operasi(object_kind::MEMORY, 200, HAK_MEMORI),
            Keputusan::Ditolak(Ditolak::OperasiTidakDikenal)
        ));
        // Negatif LINTAS OTORITAS: hak endpoint dipakai untuk operasi MEMORY.
        assert!(matches!(
            periksa_operasi(object_kind::MEMORY, dispatch::op_mem::MODIFY, right::SEND),
            Keputusan::Ditolak(Ditolak::HakTidakSahUntukKind)
        ));
        // Negatif LINTAS OTORITAS arah sebaliknya: hak memori untuk operasi TASK.
        assert!(matches!(
            periksa_operasi(object_kind::TASK, dispatch::op_task::SETPGID, right::MAP),
            Keputusan::Ditolak(Ditolak::HakTidakSahUntukKind)
        ));
        // Operasi tanpa hak wajib tetap boleh dengan hak kosong.
        assert!(matches!(
            periksa_operasi(object_kind::ENDPOINT, dispatch::op_endpoint::CLOSE, right::NONE),
            Keputusan::Diizinkan(_)
        ));
        // Token mengikat operasinya.
        assert!(match periksa_operasi(object_kind::TASK, dispatch::op_task::INFO, HAK_TASK) {
            Keputusan::Diizinkan(izin) => izin.untuk(object_kind::TASK, dispatch::op_task::INFO)
                && !izin.untuk(object_kind::MEMORY, dispatch::op_mem::INFO),
            Keputusan::Ditolak(_) => false,
        });
        assert!(!izin_dari(Keputusan::Ditolak(Ditolak::HakKurang)).is_some());
    };
}
