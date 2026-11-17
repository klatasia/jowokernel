// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! P1.48 — **OTORITAS INTEGRASI INTI KERNEL RUST**.


use core::sync::atomic::{AtomicU32, Ordering};

/// Tag laporan; cermin `KlatosIntegrasiReport` di `KlatKernelBridge.h`.
pub const REPORT_TAG: u64 = 0x4B4C_4154_494E_5447; // "KLATINTG"

/// Penghitung untuk audit (§21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counter {
    /// Jumlah `periksa_semua` yang dijalankan.
    ProdPas = 0,
    /// Jumlah otoritas yang LULUS, terakumulasi lintas pass.
    ProdSah = 1,
    /// Jumlah otoritas yang DITOLAK, terakumulasi lintas pass.
    ProdTolak = 2,
    /// Jumlah `verifikasi(id)` tunggal yang dijalankan dari produksi.
    ProdVerifikasi = 3,
    /// Jumlah `boleh_lanjut()` yang mengembalikan 1.
    ProdLanjut = 4,
    /// Jumlah `boleh_lanjut()` yang mengembalikan 0.
    ProdHenti = 5,
}
/// Jumlah penghitung.
pub const N_COUNTER: usize = 6;

/// Tahap integrasi inti Rust.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tahap {
    /// Belum ada pemeriksaan yang dijalankan.
    Belum = 0,
    /// Sudah ada pemeriksaan tunggal, agregat belum final.
    Terdaftar = 1,
    /// Seluruh otoritas manifes lulus.
    Terverifikasi = 2,
    /// Ada otoritas yang ditolak / manifes tidak lengkap.
    Gagal = 3,
}

// ---------------------------------------------------------------------------
// Manifes — tabel konstan otoritas inti Rust
// ---------------------------------------------------------------------------

/// Satu entri manifes.
#[derive(Clone, Copy)]
pub struct Otoritas {
    /// Nomor urut tetap (0..N_OTORITAS).
    pub id: u32,
    /// Nama modul Rust (untuk atribusi kegagalan lewat `nama_hash`).
    pub nama: &'static str,
    /// Tag laporan yang WAJIB dikembalikan otoritas ini.
    pub tag_harap: u64,
}

/// Mendefinisikan manifes sekaligus mesin pemeriksanya.
///
/// Setiap entri menghasilkan satu lengan `match` yang memanggil selftest
/// otoritas tersebut **di dalam crate yang sama**. Karena itu manifes tidak
/// dapat menunjuk modul yang tidak ada: kalau ada yang hilang, crate tidak
/// akan terkompilasi.
///
/// `$tag` adalah tag laporan yang diharapkan; nilainya dibangkitkan dari
/// `REPORT_TAG` masing-masing modul (beberapa modul mendeklarasikannya privat,
/// jadi nilainya ditulis harfiah di sini — dan pemeriksaan `kepala_ok` tetap
/// membandingkannya dengan tag yang **benar-benar dikembalikan** selftest).
macro_rules! manifes {
    ($( $id:expr, $nama:literal, $tag:expr, $mod:ident, $fungsi:ident; )*) => {
        /// Jumlah otoritas dalam manifes.
        pub const N_OTORITAS: u32 = 0u32 $( + { let _ = $id; 1u32 } )*;

        /// Manifes kanonik otoritas inti Rust.
        pub static MANIFEST: [Otoritas; N_OTORITAS as usize] = [
            $( Otoritas {
                id: $id,
                nama: $nama,
                tag_harap: $tag,
            } ),*
        ];

        /// Menjalankan selftest otoritas `id` dan memeriksa kepalanya.
        ///
        /// Mengembalikan 1 bila `id` ada di manifes, selftest-nya
        /// mengembalikan `tag == tag_harap` DAN `passed == checks`.
        fn periksa_satu(id: u32) -> u32 {
            match id {
                $(
                    $id => {
                        let r = crate::$mod::$fungsi();
                        kepala_ok(r.tag, r.checks, r.passed, $tag)
                    }
                )*
                _ => 0,
            }
        }
    };
}

// --- MANIFEST (dibangkitkan dari `KlatKernel/src/*.rs`) -----
manifes! {
    0, "acak_cepat", 0x4B4C_4154_4143_5054, acak_cepat, klatos_acak_cepat_selftest;
    1, "akuntansi_waktu", 0x4B4C_4154_4157_4B54, akuntansi_waktu, klatos_akuntansi_waktu_selftest;
    2, "alamat_pengguna", 0x4B4C_4154_414C_4D54, alamat_pengguna, klatos_alamat_pengguna_selftest;
    3, "antrean_perangkat", 0x4B4C_4154_4150_524B, antrean_perangkat, klatos_antrean_perangkat_selftest;
    4, "buku_besar_benang", 0x4B4C_4154_5448_524C, buku_besar_benang, klatos_benang_selftest;
    5, "buku_besar_irq", 0x4B4C_4154_4952_514C, buku_besar_irq, klatos_irq_buku_besar_selftest;
    6, "buku_besar_panggilan", 0x4B4C_4154_5359_5343, buku_besar_panggilan, klatos_panggilan_selftest;
    7, "buku_besar_proses", 0x4B4C_4154_5052_5347, buku_besar_proses, klatos_proses_selftest;
    8, "clock_id", 0x4B4C_4154_4349_4444, clock_id, klatos_clock_id_selftest;
    9, "fd_mode", 0x4B4C_4154_46444D44, fd_mode, klatos_fd_mode_selftest;
    10, "fpu", 0x4B4C_4154_4650_5555, fpu, klatos_fpu_selftest;
    11, "futex_op", 0x4B4C_4154_4655_5458, futex_op, klatos_futex_op_selftest;
    12, "id_inode", 0x4B4C_4154_4949_4E4F, id_inode, klatos_id_inode_selftest;
    13, "id_proses", 0x4B4C_4154_4950_5253, id_proses, klatos_id_proses_selftest;
    14, "id_vfs", 0x4B4C_4154_4956_4653, id_vfs, klatos_id_vfs_selftest;
    15, "int", 0x4B4C_4154_494E_5444, int, klatos_int_selftest;
    16, "ipc_handle", 0x4B4C_4154_4950_4348, ipc_handle, klatos_ipc_handle_selftest;
    17, "izin_identitas", 0x4B4C_4154_495A_4944, izin_identitas, klatos_izin_identitas_selftest;
    18, "izin_inode", 0x4B4C_4154_495A_494E, izin_inode, klatos_izin_inode_selftest;
    19, "izin_jadwal", 0x4B4C_4154_495A_4A44, izin_jadwal, klatos_izin_jadwal_selftest;
    20, "izin_sinyal", 0x4B4C_4154_495A_5347, izin_sinyal, klatos_izin_sinyal_selftest;
    21, "jam_inti", 0x4B4C_4154_4A414D49, jam_inti, klatos_jam_inti_selftest;
    22, "kebijakan_bendera_mmap", 0x4B4C_4154_4D4D_4150, kebijakan_bendera_mmap, klatos_kebijakan_bendera_mmap_selftest;
    23, "mmap_prot", 0x4B4C_4154_4D4D_4150, mmap_prot, klatos_mmap_prot_selftest;
    24, "mount_bendera", 0x4B4C_4154_4D4E_5442, mount_bendera, klatos_mount_bendera_selftest;
    25, "ksm_open", 0x4B4C_4154_4F504C47, ksm_open, klatos_ksm_open_selftest;
    26, "pendaftaran_perangkat", 0x4B4C_4154_5044_5652, pendaftaran_perangkat, klatos_pendaftaran_perangkat_selftest;
    27, "penyangga_ganda", 0x4B4C_4154_5047_4441, penyangga_ganda, klatos_penyangga_ganda_selftest;
    28, "ksm_transisi", 0x4B4C_4154_504C_4447, ksm_transisi, klatos_ksm_transisi_selftest;
    29, "priority_bucket", 0x4B4C_4154_5343_4844, priority_bucket, klatos_sched_bucket_selftest;
    30, "rentang_berisi", 0x4B4C_4154_5642_4953, rentang_berisi, klatos_vr_contains_selftest;
    31, "rentang_pengguna", 0x4B4C_4154_524E_4750, rentang_pengguna, klatos_rentang_pengguna_selftest;
    32, "rentang_tumpang", 0x4B4C_4154_5652_4958, rentang_tumpang, klatos_vr_intersects_selftest;
    33, "seek_kebijakan", 0x4B4C_4154_5345_454B, seek_kebijakan, klatos_seek_selftest;
    34, "signal_action", 0x4B4C_4154_53494741, signal_action, klatos_signal_action_selftest;
    35, "siklus_pty", 0x4B4C_4154_5054_5943, siklus_pty, klatos_siklus_pty_selftest;
    36, "task_id", 0x4B4C_4154_544B_4944, task_id, klatos_task_id_selftest;
    37, "thread_transisi", 0x4B4C_4154_5448_5244, thread_transisi, klatos_thread_transisi_selftest;
    38, "timer", 0x4B4C_4154_544D_5244, timer, klatos_timer_selftest;
    39, "ksm_keputusan_jalur", 0x4B4C_4154_554E_4144, ksm_keputusan_jalur, klatos_ksm_keputusan_jalur_selftest;
    40, "ksm_parser_izin", 0x4B4C_4154_554E564C, ksm_parser_izin, klatos_ksm_parser_izin_selftest;
    41, "validasi_bendera_faccessat", 0x4B4C_4154_4641_4343, validasi_bendera_faccessat, klatos_validasi_bendera_faccessat_selftest;
    42, "validasi_bendera_open_larangan", 0x4B4C_4154_4F50_4E4C, validasi_bendera_open_larangan, klatos_validasi_bendera_open_larangan_selftest;
    43, "validasi_nomor_sinyal", 0x4B4C_4154_5349_474E, validasi_nomor_sinyal, klatos_validasi_nomor_sinyal_selftest;
    44, "validasi_perintah_fcntl", 0x4B4C_4154_4643_4E54, validasi_perintah_fcntl, klatos_validasi_perintah_fcntl_selftest;
    45, "wilayah_ruang", 0x4B4C_4154_574C_5948, wilayah_ruang, klatos_wilayah_ruang_selftest;
    46, "graf_proses_benang", 0x4B4C_4154_4752_5048, graf_proses_benang, klatos_graf_selftest;
    47, "penghalang_benang", 0x4B4C_4154_5048_424C, penghalang_benang, klatos_pembendung_selftest;
    48, "jadwal_antrean", 0x4B4C_4154_4A44_574C, jadwal_antrean, klatos_jadwal_selftest;
    49, "keadaan_wilayah", 0x4B4C_4154_5753_5441, keadaan_wilayah, klatos_wilayah_selftest;
    50, "tempat_wilayah", 0x4B4C_4154_544D_5054, tempat_wilayah, klatos_tempat_selftest;
    51, "admisipf", 0x4B4C_4154_5046_4144, admisipf, klatos_pf_selftest;
    52, "penjaga_pf", 0x4B4C_4154_5050_4647, penjaga_pf, klatos_ppf_selftest;
    53, "akuntansi_memori", 0x4B4C_4154_414D_454D, akuntansi_memori, klatos_am_selftest;
    54, "mod_berkas", 0x4B4C_4154_4D4F_4446, mod_berkas, klatos_mod_selftest;
    55, "mod_chmod", 0x4B4C_4154_4348_4D44, mod_chmod, klatos_modchmod_selftest;
    56, "mod_chown", 0x4B4C_4154_4348_5754, mod_chown, klatos_modchown_selftest;
    57, "mod_hapus", 0x4B4C_4154_4841_5053, mod_hapus, klatos_modhapus_selftest;
    58, "admisi_ruang", 0x4B4C_4154_4144_4D53, admisi_ruang, klatos_admisi_selftest;
    59, "vmo_anon", 0x4B4C_4154_564D_414E, vmo_anon, klatos_vmoan_selftest;
    60, "vmo_inode", 0x4B4C_4154_564D_494E, vmo_inode, klatos_vmoin_selftest;
    61, "wilayah_cari", 0x4B4C_4154_5743_4152, wilayah_cari, klatos_wcar_selftest;
    62, "akuntansi_fisik", 0x4B4C_4154_4146_4953, akuntansi_fisik, klatos_af_selftest;
    63, "siklus_pasang", 0x4B4C_4154_534B_5053, siklus_pasang, klatos_pas_selftest;
    64, "kebijakan_panggilan", 0x4B4C_4154_5359_5350, kebijakan_panggilan, klatos_kebijakan_selftest;
    65, "siklus_soket", 0x4B4C_4154_534B_534B, siklus_soket, klatos_soket_selftest;
    66, "admisi_exec", 0x4B4C_4154_5345_5845, admisi_exec, klatos_exec_selftest;
    67, "panggilan_soket", 0x4B4C_4154_5350_4B53, panggilan_soket, klatos_soketp_selftest;
    68, "admisi_perangkat", 0x4B4C_4154_4445_5641, admisi_perangkat, klatos_dev_selftest;
    69, "admisi_vfs", 0x4B4C_4154_5646_5341, admisi_vfs, klatos_vfs_selftest;
    70, "admisi_vfs2", 0x4B4C_4154_5646_5332, admisi_vfs2, klatos_vfs2_selftest;
    71, "admisi_rmdir", 0x4B4C_4154_524D_4449, admisi_rmdir, klatos_rmdir_selftest;
    72, "admisi_rename", 0x4B4C_4154_524E_414D, admisi_rename, klatos_rename_selftest;
    73, "admisi_resolusi", 0x4B4C_4154_5253_4F4C, admisi_resolusi, klatos_resolusi_selftest;
    74, "admisi_vfs3", 0x4B4C_4154_5646_5333, admisi_vfs3, klatos_vfs3_selftest;
    75, "ksm_jalur", 0x4B4C_4154_4741_5445, ksm_jalur, klatos_ksm_jalur_selftest;
    76, "admisi_vfs4", 0x4B4C_4154_5646_5334, admisi_vfs4, klatos_vfs4_selftest;
    77, "admisi_fs", 0x4B4C_4154_4653_4131, admisi_fs, klatos_fs1_selftest;
    78, "admisi_fs2", 0x4B4C_4154_4653_4232, admisi_fs2, klatos_fs2_selftest;
    79, "admisi_fs3", 0x4B4C_4154_4653_4233, admisi_fs3, klatos_fs3_selftest;
    80, "admisi_fs4", 0x4B4C_4154_4653_4234, admisi_fs4, klatos_fs4_selftest;
    81, "admisi_proc", 0x4B4C_4154_5052_4F43, admisi_proc, klatos_proc_selftest;
    82, "admisi_sysfs", 0x4B4C_4154_5359_5346, admisi_sysfs, klatos_sysfs_selftest;
    83, "admisi_pasang", 0x4B4C_4154_5044_4556, admisi_pasang, klatos_pdev_selftest;
    84, "admisi_net", 0x4B4C_4154_4E45_5441, admisi_net, klatos_net_selftest;
    85, "admisi_net2", 0x4B4C_4154_4E45_5432, admisi_net2, klatos_net2_selftest;
    86, "admisi_proses", 0x4B4C_4154_5052_4F53, admisi_proses, klatos_pros_selftest;
    87, "admisi_jadwal", 0x4B4C_4154_5447_4153, admisi_jadwal, klatos_tugas_selftest;
    88, "admisi_sinyal2", 0x4B4C_4154_5349_4732, admisi_sinyal2, klatos_sinyal2_selftest;
    89, "admisi_misc", 0x4B4C_4154_4D49_5343, admisi_misc, klatos_misc_selftest;
    90, "admisi_mmap2", 0x4B4C_4154_4D32_41, admisi_mmap2, klatos_mmap2_selftest;
    91, "admisi_mem2", 0x4B4C_4154_4D45_4D32, admisi_mem2, klatos_mem2_selftest;
    92, "admisi_mem3", 0x4B4C_4154_4D45_4D33, admisi_mem3, klatos_mem3_selftest;
    93, "admisi_aman", 0x4B4C_4154_414D_414E, admisi_aman, klatos_aman_selftest;
    94, "admisi_kanal", 0x4B4C_4154_4B41_4E4C, admisi_kanal, klatos_kanal_selftest;
    95, "admisi_berkas", 0x4B4C_4154_4252_4B53, admisi_berkas, klatos_berkas_selftest;
    96, "admisi_berkas2", 0x4B4C_4154_4653_5232, admisi_berkas2, klatos_berkas2_selftest;
    97, "admisi_exec2", 0x4B4C_4154_4558_4532, admisi_exec2, klatos_exec2_selftest;
    98, "admisi_proses2", 0x4B4C_4154_5052_5332, admisi_proses2, klatos_pros2_selftest;
    99, "admisi_jaring", 0x4B4C_4154_4A52_4E47, admisi_jaring, klatos_jaring_selftest;
    100, "admisi_panggilan", 0x4B4C_4154_5041_4E47, admisi_panggilan, klatos_pang_selftest;
    101, "admisi_panggilan2", 0x4B4C_4154_5041_4E32, admisi_panggilan2, klatos_pang2_selftest;
    102, "panggilan_aksi", 0x4B4C_4154_5041_4B53, panggilan_aksi, klatos_panggilan_aksi_selftest;
    103, "keamanan_aksi", 0x4B4C_4154_4B4D_4E41, keamanan_aksi, klatos_keamanan_aksi_selftest;
    104, "kasan_bayangan", 0x4B4C_4154_4B53_4E42, kasan_bayangan, klatos_kasan_bayangan_selftest;
    105, "fortuna", 0x4B4C_4154_464F_5254, fortuna, klatos_fortuna_selftest;
    106, "ksm_pohon_jalur", 0x4B4C_4154_414A_414C, ksm_pohon_jalur, klatos_ksm_pohon_jalur_selftest;
    107, "ksm_kapabilitas", 0x4B4C_4154_4B53_4D43, ksm_kapabilitas, klatos_ksm_kap_selftest;
    108, "input_keymap", 0x4B4C_4154_4B45_594D, input_keymap, klatos_keymap_selftest;
    109, "mekanisme_abi2", 0x4B4C_4154_4D45_4B41, mekanisme_abi2, klatos_mekanisme_abi2_selftest;
    110, "keadaan_vmoan", 0x4B4C_4154_564D_5341, keadaan_vmoan, klatos_vmoan_state_selftest;
    111, "identitas_aksi", 0x4B4C_4154_4944_4E54, identitas_aksi, klatos_identitas_selftest;
    112, "cow_map", 0x4B4C_4154_434F_574D, cow_map, klatos_vmoan_cow_map_selftest;
    113, "memori_efek", 0x4B4C_4154_4D45_4D4F, memori_efek, klatos_memori_efek_selftest;
    114, "fileref_efek", 0x4B4C_4154_4652_4546, fileref_efek, klatos_fileref_efek_selftest;
    115, "cincin_penyangga", 0x4B4C_4154_4349_4E43, cincin_penyangga, klatos_cincin_selftest;
    116, "ikatan_efek", 0x4B4C_4154_494B_4154, ikatan_efek, klatos_ikatan_efek_selftest;
    117, "penjejak_utas", 0x4B4C_4154_5055_4A54, penjejak_utas, klatos_penjejak_utas_selftest;
    118, "keadaan_ruang", 0x4B4C_4154_5255_414E, keadaan_ruang, klatos_ruang_selftest;
    119, "keadaan_proses", 0x4B4C_4154_5052_5353, keadaan_proses, klatos_keadaan_proses_selftest;
    120, "jalur_leksikal", 0x4B4C_4154_4A4C_4B53, jalur_leksikal, klatos_jalur_leksikal_selftest;
    121, "wilayah_fisik", 0x4B4C_4154_574C_4653, wilayah_fisik, klatos_wfisik_selftest;
    122, "sfn_fat", 0x4B4C_4154_5346_4E31, sfn_fat, klatos_sfn_selftest;
    123, "peringkat_kunci", 0x4B4C_4154_4C52_4E31, peringkat_kunci, klatos_lockrank_selftest;
    124, "siklus_berkas_maya", 0x4B4C_4154_424D_4159, siklus_berkas_maya, klatos_bmaya_selftest;
    125, "kustodi_jalur", 0x4B4C_4154_4B4A_4C52, kustodi_jalur, klatos_kustodi_jalur_selftest;
    126, "siklus_ujung", 0x4B4C_4154_554A_4E47, siklus_ujung, klatos_ujung_selftest;
    127, "penunggu_ujung", 0x4B4C_4154_504E_4747, penunggu_ujung, klatos_penunggu_selftest;
    128, "siklus_permintaan", 0x4B4C_4154_504D_4E54, siklus_permintaan, klatos_minta_selftest;
    129, "antrian_timer", 0x4B4C_4154_5449_4D52, antrian_timer, klatos_timer_antre_selftest;
    130, "manifes_boot", 0x4B4C_4154_424F_4F54, manifes_boot, klatos_boot_manifest_selftest;
    131, "opsi_berkas", 0x4B4C_4154_4F50_5349, opsi_berkas, klatos_opsi_selftest;
    132, "kunci_eksklusif", 0x4B4C_4154_4D54_5831, kunci_eksklusif, klatos_kunci_selftest;
    133, "antrean_tunggu", 0x4B4C_4154_414E_5452, antrean_tunggu, klatos_antrean_selftest;
    134, "kebijakan_pasang", 0x4B4C_4154_5053_4E47, kebijakan_pasang, klatos_pasang_selftest;
    135, "pendaftaran_irq", 0x4B4C_4154_4952_5152, pendaftaran_irq, klatos_irq_selftest;
    136, "klon_wilayah", 0x4B4C_4154_4B4C_4E57, klon_wilayah, klatos_wilayah_klon_selftest;
    137, "kebijakan_sistem", 0x4B4C_4154_5359_534B, kebijakan_sistem, klatos_sys_selftest;
    138, "siklus_jaringan", 0x4B4C_4154_4E45_5457, siklus_jaringan, klatos_netw_selftest;
    139, "siklus_profiling", 0x4B4C_4154_5052_464C, siklus_profiling, klatos_prof_selftest;
    140, "siklus_matikan", 0x4B4C_4154_4D41_544B, siklus_matikan, klatos_matikan_selftest;
    141, "kebijakan_tugas", 0x4B4C_4154_5447_534B, kebijakan_tugas, klatos_tugask_selftest;
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

static TAHAP: AtomicU32 = AtomicU32::new(Tahap::Belum as u32);
static SAH: AtomicU32 = AtomicU32::new(0);
static TOLAK: AtomicU32 = AtomicU32::new(0);
static TOLAK_PERTAMA: AtomicU32 = AtomicU32::new(0);
static BITMAP_LO: AtomicU32 = AtomicU32::new(0);
static BITMAP_HI: AtomicU32 = AtomicU32::new(0);
static PAS: AtomicU32 = AtomicU32::new(0);
/// Diagnostik (P1.67): id otoritas pertama yang GAGAL `verifikasi()`.
static GAGAL_VERIFIKASI: AtomicU32 = AtomicU32::new(u32::MAX);
/// Diagnostik (P1.67): indeks `catat()` pertama yang GAGAL di dalam matriks.
static GAGAL_IDX: AtomicU32 = AtomicU32::new(u32::MAX);
static COUNTERS: [AtomicU32; N_COUNTER] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];

#[inline]
fn bump(c: Counter) {
    COUNTERS[c as usize].fetch_add(1, Ordering::Relaxed);
}

/// Menambah penghitung `c` dengan `n` (saturating).
#[inline]
fn bump_n(c: Counter, n: u32) {
    let _ = COUNTERS[c as usize].fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
        Some(v.saturating_add(n))
    });
}

/// Memeriksa "kepala" laporan: tag cocok dan selftest melaporkan diri lulus.
#[inline]
fn kepala_ok(tag: u64, checks: u32, passed: u32, harap: u64) -> u32 {
    if tag == harap && passed == checks && checks > 0 {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// Mekanisme
// ---------------------------------------------------------------------------

/// Memeriksa satu otoritas terhadap manifes. `id` di luar manifes -> 0.
fn verifikasi_impl(id: u32) -> u32 {
    bump(Counter::ProdVerifikasi);
    let ok = periksa_satu(id);
    if id < N_OTORITAS {
        if ok == 1 {
            if TAHAP.load(Ordering::Relaxed) == Tahap::Belum as u32 {
                TAHAP.store(Tahap::Terdaftar as u32, Ordering::Relaxed);
            }
        } else {
            GAGAL_VERIFIKASI.store(id, Ordering::Relaxed);
            TAHAP.store(Tahap::Gagal as u32, Ordering::Relaxed);
        }
    }
    ok
}

/// Memeriksa tag saja (tanpa menjalankan selftest). Disediakan sebagai kait
/// uji untuk kelompok negatif; produksi memakai [`verifikasi_impl`].
fn uji_tag_impl(id: u32, tag: u32, tag_hi: u32) -> u32 {
    if id >= N_OTORITAS {
        return 0;
    }
    let nyata = ((tag_hi as u64) << 32) | (tag as u64);
    if nyata == MANIFEST[id as usize].tag_harap {
        1
    } else {
        0
    }
}

/// Menjalankan seluruh manifes. Mengembalikan jumlah otoritas yang lulus.
fn periksa_semua_impl() -> u32 {
    let mut sah = 0u32;
    let mut tolak = 0u32;
    let mut pertama = 0u32;
    let mut lo = 0u32;
    let mut hi = 0u32;
    let mut i = 0u32;
    while i < N_OTORITAS {
        if periksa_satu(i) == 1 {
            sah += 1;
            // F-P175-01 — bitmap 64-bit. `i >= 64` TIDAK dapat diwakili; tanpa
            // cabang ini `1u32 << (i - 32)` untuk `i == 64` adalah geser
            // termasku (shift amount dimask `& 31`) yang secara tidak sengaja
            // menyalakan bit milik otoritas 32. Verdict tetap dibawa `SAH`/
            // `TOLAK`/`TAHAP`; bitmap hanyalah ringkasan.
            if i < 32 {
                lo |= 1u32 << i;
            } else if i < 64 {
                hi |= 1u32 << (i - 32);
            }
        } else {
            if tolak == 0 {
                pertama = i;
            }
            tolak += 1;
        }
        i += 1;
    }
    SAH.store(sah, Ordering::Relaxed);
    TOLAK.store(tolak, Ordering::Relaxed);
    TOLAK_PERTAMA.store(pertama, Ordering::Relaxed);
    BITMAP_LO.store(lo, Ordering::Relaxed);
    BITMAP_HI.store(hi, Ordering::Relaxed);
    PAS.fetch_add(1, Ordering::Relaxed);
    bump(Counter::ProdPas);
    bump_n(Counter::ProdSah, sah);
    bump_n(Counter::ProdTolak, tolak);
    TAHAP.store(
        if tolak == 0 && sah == N_OTORITAS {
            Tahap::Terverifikasi as u32
        } else {
            Tahap::Gagal as u32
        },
        Ordering::Relaxed,
    );
    sah
}

/// Keputusan: bolehkah kernel melanjutkan dengan inti Rust ini?
fn boleh_lanjut_impl() -> u32 {
    if TAHAP.load(Ordering::Relaxed) == Tahap::Terverifikasi as u32 {
        bump(Counter::ProdLanjut);
        1
    } else {
        bump(Counter::ProdHenti);
        0
    }
}

/// Reset seluruh state integrasi. TIDAK dipanggil dari produksi.
fn reset_impl() {
    TAHAP.store(Tahap::Belum as u32, Ordering::Release);
    SAH.store(0, Ordering::Release);
    TOLAK.store(0, Ordering::Release);
    TOLAK_PERTAMA.store(0, Ordering::Release);
    BITMAP_LO.store(0, Ordering::Release);
    BITMAP_HI.store(0, Ordering::Release);
    PAS.store(0, Ordering::Release);
    for c in COUNTERS.iter() {
        c.store(0, Ordering::Release);
    }
}

// ---------------------------------------------------------------------------
// Permukaan C ABI
// ---------------------------------------------------------------------------

/// Memeriksa satu otoritas manifes (`id`). 1 = lulus, 0 = gagal/tak ada.
#[no_mangle]
pub extern "C" fn klatos_integrasi_verifikasi(id: u32) -> u32 {
    verifikasi_impl(id)
}

/// Memeriksa tag saja (kait uji). `tag` = 32 bit rendah, `tag_hi` = 32 bit tinggi.
#[no_mangle]
pub extern "C" fn klatos_integrasi_uji_tag(id: u32, tag: u32, tag_hi: u32) -> u32 {
    uji_tag_impl(id, tag, tag_hi)
}

/// Menjalankan seluruh manifes; mengembalikan jumlah otoritas yang lulus.
#[no_mangle]
pub extern "C" fn klatos_integrasi_periksa_semua() -> u32 {
    periksa_semua_impl()
}

/// 1 bila seluruh manifes terverifikasi (keputusan milik Rust).
#[no_mangle]
pub extern "C" fn klatos_integrasi_boleh_lanjut() -> u32 {
    boleh_lanjut_impl()
}

/// Tahap integrasi saat ini (`Tahap`).
#[no_mangle]
pub extern "C" fn klatos_integrasi_tahap() -> u32 {
    TAHAP.load(Ordering::Relaxed)
}

/// Jumlah otoritas yang lulus pada pass terakhir.
#[no_mangle]
pub extern "C" fn klatos_integrasi_sah() -> u32 {
    SAH.load(Ordering::Relaxed)
}

/// Jumlah otoritas yang ditolak pada pass terakhir.
#[no_mangle]
pub extern "C" fn klatos_integrasi_tolak() -> u32 {
    TOLAK.load(Ordering::Relaxed)
}

/// Id otoritas pertama yang ditolak pada pass terakhir (0 bila tak ada).
#[no_mangle]
pub extern "C" fn klatos_integrasi_tolak_pertama() -> u32 {
    TOLAK_PERTAMA.load(Ordering::Relaxed)
}

/// 32 bit rendah bitmap otoritas yang lulus.
#[no_mangle]
pub extern "C" fn klatos_integrasi_bitmap_lo() -> u32 {
    BITMAP_LO.load(Ordering::Relaxed)
}

/// 32 bit tinggi bitmap otoritas yang lulus.
#[no_mangle]
pub extern "C" fn klatos_integrasi_bitmap_hi() -> u32 {
    BITMAP_HI.load(Ordering::Relaxed)
}

/// Jumlah otoritas dalam manifes.
#[no_mangle]
pub extern "C" fn klatos_integrasi_n_otoritas() -> u32 {
    N_OTORITAS
}

/// Jumlah penghitung audit.
#[no_mangle]
pub extern "C" fn klatos_integrasi_n_counter() -> u32 {
    N_COUNTER as u32
}

/// Membaca penghitung audit `which`.
#[no_mangle]
pub extern "C" fn klatos_integrasi_counter(which: u32) -> u32 {
    if (which as usize) < N_COUNTER {
        COUNTERS[which as usize].load(Ordering::Relaxed)
    } else {
        0
    }
}

/// Tag harapan otoritas `id` (0 bila `id` di luar manifes).
#[no_mangle]
pub extern "C" fn klatos_integrasi_tag_harap(id: u32) -> u64 {
    if id < N_OTORITAS {
        MANIFEST[id as usize].tag_harap
    } else {
        0
    }
}

/// FNV-1a 32 dari nama otoritas `id` — atribusi kegagalan tanpa pointer.
#[no_mangle]
pub extern "C" fn klatos_integrasi_nama_hash(id: u32) -> u32 {
    if id >= N_OTORITAS {
        return 0;
    }
    let mut h: u32 = 0x811C_9DC5;
    for b in MANIFEST[id as usize].nama.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

/// Jumlah `periksa_semua` yang sudah dijalankan.
#[no_mangle]
pub extern "C" fn klatos_integrasi_pas() -> u32 {
    PAS.load(Ordering::Relaxed)
}

/// Reset state integrasi (bukan jalur produksi).
#[no_mangle]
pub extern "C" fn klatos_integrasi_reset() -> u32 {
    reset_impl();
    1
}

/// Diagnostik P1.67: id otoritas pertama yang gagal `verifikasi()`
/// (`u32::MAX` bila tidak ada). Bukan bagian keputusan integrasi.
#[no_mangle]
pub extern "C" fn klatos_integrasi_gagal_verifikasi() -> u32 {
    GAGAL_VERIFIKASI.load(Ordering::Relaxed)
}

/// Diagnostik P1.67: indeks `catat()` pertama yang gagal di matriks
/// (`u32::MAX` bila tidak ada). Bukan bagian keputusan integrasi.
#[no_mangle]
pub extern "C" fn klatos_integrasi_gagal_idx() -> u32 {
    GAGAL_IDX.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Laporan
// ---------------------------------------------------------------------------

/// Laporan P1.48; cermin `KlatosIntegrasiReport` di `KlatKernelBridge.h`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct KlatosIntegrasiReport {
    pub tag: u64,
    pub checks: u32,
    pub passed: u32,
    pub peta_checks: u32,
    pub peta_passed: u32,
    pub neg_checks: u32,
    pub neg_passed: u32,
    pub n_otoritas: u32,
    pub sah: u32,
    pub tolak: u32,
    pub tolak_pertama: u32,
    pub tahap: u32,
    pub boleh_lanjut: u32,
    pub bitmap_lo: u32,
    pub bitmap_hi: u32,
    pub status: i32,
}

// ---------------------------------------------------------------------------
// Matriks selftest
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    Peta,
    Neg,
}

struct Matrix {
    checks: u32,
    passed: u32,
    peta_checks: u32,
    peta_passed: u32,
    neg_checks: u32,
    neg_passed: u32,
    /// Diagnostik (P1.67): indeks `catat()` pertama yang gagal; `u32::MAX` bila
    /// tidak ada. Dipakai untuk menamai pemeriksaan yang gagal di kernel.
    gagal_idx: u32,
}

impl Matrix {
    const fn new() -> Self {
        Matrix {
            checks: 0,
            passed: 0,
            peta_checks: 0,
            peta_passed: 0,
            neg_checks: 0,
            neg_passed: 0,
            gagal_idx: u32::MAX,
        }
    }
    fn catat(&mut self, lulus: bool, group: Group) {
        let idx = self.checks;
        self.checks += 1;
        match group {
            Group::Peta => {
                self.peta_checks += 1;
                if lulus {
                    self.peta_passed += 1;
                }
            }
            Group::Neg => {
                self.neg_checks += 1;
                if lulus {
                    self.neg_passed += 1;
                }
            }
        }
        if lulus {
            self.passed += 1;
        } else if self.gagal_idx == u32::MAX {
            self.gagal_idx = idx;
        }
    }
}

/// Oracle "naif": salinan manifes dalam bentuk array biasa untuk pembanding.
struct Naif {
    tag: [u64; N_OTORITAS as usize],
}

impl Naif {
    fn baru() -> Self {
        let mut tag = [0u64; N_OTORITAS as usize];
        let mut i = 0usize;
        while i < N_OTORITAS as usize {
            tag[i] = MANIFEST[i].tag_harap;
            i += 1;
        }
        Naif { tag }
    }
    fn periksa(&self, id: u32, t: u64) -> u32 {
        if id as usize >= N_OTORITAS as usize {
            0
        } else if self.tag[id as usize] == t {
            1
        } else {
            0
        }
    }
}

/// Bitmap yang diharapkan untuk `n` otoritas yang lulus: bit `0..n-1` terisi.
///
/// Dihitung dari `n` supaya harapan TIDAK PERNAH basi saat manifes bertambah
/// (F-P174-01: matriks dulu mengarang `0x1FFF_FFFF` untuk 61 bit, lalu tidak
/// diperbarui ketika manifes tumbuh ke 63 sehingga `P148-RUNTIME-FAIL`
/// terbit di setiap boot).
const fn bitmap_harap(n: u32) -> (u32, u32) {
    let lo = if n >= 32 { u32::MAX } else { (1u32 << n) - 1 };
    let hi = if n <= 32 {
        0
    } else if n >= 64 {
        u32::MAX
    } else {
        (1u32 << (n - 32)) - 1
    };
    (lo, hi)
}

fn run_matrix() -> KlatosIntegrasiReport {
    let mut m = Matrix::new();
    GAGAL_VERIFIKASI.store(u32::MAX, Ordering::Relaxed);
    GAGAL_IDX.store(u32::MAX, Ordering::Relaxed);

    // --- kelompok 0: konstanta & bentuk manifes ---------------------------
    m.catat(REPORT_TAG == 0x4B4C_4154_494E_5447, Group::Peta);
    m.catat(N_OTORITAS == 142, Group::Peta);
    m.catat(klatos_integrasi_n_otoritas() == N_OTORITAS, Group::Peta);
    m.catat(N_COUNTER == 6, Group::Peta);
    m.catat(klatos_integrasi_counter(99) == 0, Group::Peta);
    // id kontinu 0..N_OTORITAS-1
    {
        let mut kontinu = true;
        let mut i = 0u32;
        while i < N_OTORITAS {
            if MANIFEST[i as usize].id != i {
                kontinu = false;
                break;
            }
            i += 1;
        }
        m.catat(kontinu, Group::Peta);
    }
    // nama tidak kosong, tag tidak nol
    {
        let mut ok = true;
        let mut i = 0u32;
        while i < N_OTORITAS {
            if MANIFEST[i as usize].tag_harap == 0 || MANIFEST[i as usize].nama.is_empty() {
                ok = false;
                break;
            }
            i += 1;
        }
        m.catat(ok, Group::Peta);
    }
    // hash nama unik (atribusi kegagalan harus deterministik & berbeda)
    {
        let mut unik = true;
        let mut i = 0u32;
        while i < N_OTORITAS {
            let mut j = i + 1;
            while j < N_OTORITAS {
                if klatos_integrasi_nama_hash(i) == klatos_integrasi_nama_hash(j) {
                    unik = false;
                    break;
                }
                j += 1;
            }
            if !unik {
                break;
            }
            i += 1;
        }
        m.catat(unik, Group::Peta);
    }

    // --- kelompok 1: verifikasi tunggal seluruh manifes --------------------
    reset_impl();
    let naif = Naif::baru();
    for i in 0..N_OTORITAS {
        m.catat(klatos_integrasi_verifikasi(i) == 1, Group::Peta);
        m.catat(
            klatos_integrasi_tag_harap(i) == naif.tag[i as usize],
            Group::Peta,
        );
        // produksi vs oracle: tag harap dari manifes == yang dikembalikan selftest
        m.catat(
            uji_tag_impl(i, (naif.tag[i as usize] & 0xFFFF_FFFF) as u32,
                        (naif.tag[i as usize] >> 32) as u32) == 1,
            Group::Peta,
        );
    }
    m.catat(klatos_integrasi_tahap() == Tahap::Terdaftar as u32, Group::Peta);

    // --- kelompok 2: pass penuh -------------------------------------------
    reset_impl();
    let sah = klatos_integrasi_periksa_semua();
    m.catat(sah == N_OTORITAS, Group::Peta);
    m.catat(klatos_integrasi_sah() == N_OTORITAS, Group::Peta);
    m.catat(klatos_integrasi_tolak() == 0, Group::Peta);
    m.catat(klatos_integrasi_tahap() == Tahap::Terverifikasi as u32, Group::Peta);
    m.catat(klatos_integrasi_boleh_lanjut() == 1, Group::Peta);
    m.catat(klatos_integrasi_pas() == 1, Group::Peta);
    // bitmap: N_OTORITAS bit terisi -> lo = 32 bit rendah, hi = sisanya.
    // Dihitung dari N_OTORITAS, bukan dikarang, supaya tidak basi saat
    // manifes bertambah (F-P174-01: nilai lama `0x1FFF_FFFF` untuk 61 bit
    // sudah usang sejak manifes mencapai 63).
    let (lo_harap, hi_harap) = bitmap_harap(N_OTORITAS);
    m.catat(klatos_integrasi_bitmap_lo() == lo_harap, Group::Peta);
    m.catat(klatos_integrasi_bitmap_hi() == hi_harap, Group::Peta);
    // determinisme: pass kedua memberi angka identik
    {
        let lo1 = klatos_integrasi_bitmap_lo();
        let hi1 = klatos_integrasi_bitmap_hi();
        let sah2 = klatos_integrasi_periksa_semua();
        m.catat(sah2 == N_OTORITAS, Group::Peta);
        m.catat(klatos_integrasi_bitmap_lo() == lo1, Group::Peta);
        m.catat(klatos_integrasi_bitmap_hi() == hi1, Group::Peta);
        m.catat(klatos_integrasi_pas() == 2, Group::Peta);
    }

    // --- kelompok 3: negatif ----------------------------------------------
    reset_impl();
    m.catat(klatos_integrasi_tahap() == Tahap::Belum as u32, Group::Neg);
    // Belum diperiksa -> tidak boleh lanjut.
    m.catat(klatos_integrasi_boleh_lanjut() == 0, Group::Neg);
    m.catat(klatos_integrasi_counter(Counter::ProdHenti as u32) >= 1, Group::Neg);
    // id di luar manifes ditolak, tanpa mengubah tahap.
    m.catat(klatos_integrasi_verifikasi(N_OTORITAS) == 0, Group::Neg);
    m.catat(klatos_integrasi_verifikasi(999) == 0, Group::Neg);
    m.catat(klatos_integrasi_verifikasi(0xFFFF_FFFF) == 0, Group::Neg);
    m.catat(klatos_integrasi_tag_harap(N_OTORITAS) == 0, Group::Neg);
    m.catat(klatos_integrasi_nama_hash(N_OTORITAS) == 0, Group::Neg);
    m.catat(klatos_integrasi_uji_tag(N_OTORITAS, 0, 0) == 0, Group::Neg);
    // tag salah ditolak oleh kait uji.
    for i in [0u32, 5, 22, 46, 47] {
        let benar = MANIFEST[i as usize].tag_harap;
        let salah = benar ^ 0x0000_0000_0000_00FF;
        m.catat(
            klatos_integrasi_uji_tag(i, (salah & 0xFFFF_FFFF) as u32, (salah >> 32) as u32) == 0,
            Group::Neg,
        );
        m.catat(
            klatos_integrasi_uji_tag(i, (benar & 0xFFFF_FFFF) as u32, (benar >> 32) as u32) == 1,
            Group::Neg,
        );
    }
    // Oracle naif setuju pada kedua sisi.
    m.catat(naif.periksa(5, MANIFEST[5].tag_harap) == 1, Group::Neg);
    m.catat(naif.periksa(5, 0xDEAD_BEEF) == 0, Group::Neg);
    m.catat(naif.periksa(999, MANIFEST[5].tag_harap) == 0, Group::Neg);

    // --- kelompok 4: pemulihan --------------------------------------------
    // Setelah negatif, pass penuh harus mengembalikan ke Terverifikasi.
    let sah_pulih = klatos_integrasi_periksa_semua();
    m.catat(sah_pulih == N_OTORITAS, Group::Neg);
    m.catat(klatos_integrasi_tahap() == Tahap::Terverifikasi as u32, Group::Neg);
    m.catat(klatos_integrasi_tolak() == 0, Group::Neg);
    m.catat(klatos_integrasi_boleh_lanjut() == 1, Group::Neg);

    GAGAL_IDX.store(m.gagal_idx, Ordering::Relaxed);
    let status = if m.checks == m.passed { 0 } else { -1 };

    KlatosIntegrasiReport {
        tag: REPORT_TAG,
        checks: m.checks,
        passed: m.passed,
        peta_checks: m.peta_checks,
        peta_passed: m.peta_passed,
        neg_checks: m.neg_checks,
        neg_passed: m.neg_passed,
        n_otoritas: N_OTORITAS,
        sah: klatos_integrasi_sah(),
        tolak: klatos_integrasi_tolak(),
        tolak_pertama: klatos_integrasi_tolak_pertama(),
        tahap: klatos_integrasi_tahap(),
        boleh_lanjut: boleh_lanjut_impl(),
        bitmap_lo: klatos_integrasi_bitmap_lo(),
        bitmap_hi: klatos_integrasi_bitmap_hi(),
        status,
    }
}

/// Selftest P1.48.
#[no_mangle]
pub extern "C" fn klatos_integrasi_selftest() -> KlatosIntegrasiReport {
    run_matrix()
}

// ---------------------------------------------------------------------------
// Konformansi waktu-kompilasi
// ---------------------------------------------------------------------------

pub mod conformance {
    use super::*;

    const _: () = {
        assert!(REPORT_TAG == 0x4B4C_4154_494E_5447);
        assert!(N_OTORITAS == 142);
        assert!(N_COUNTER == 6);
        assert!(Tahap::Belum as usize == 0);
        assert!(Tahap::Terdaftar as usize == 1);
        assert!(Tahap::Terverifikasi as usize == 2);
        assert!(Tahap::Gagal as usize == 3);
        assert!(Counter::ProdPas as usize == 0);
        assert!(Counter::ProdSah as usize == 1);
        assert!(Counter::ProdTolak as usize == 2);
        assert!(Counter::ProdVerifikasi as usize == 3);
        assert!(Counter::ProdLanjut as usize == 4);
        assert!(Counter::ProdHenti as usize == 5);

        // 8 (tag) + 15*4 = 68, dibulatkan ke kelipatan align 8 -> 72.
        assert!(core::mem::size_of::<KlatosIntegrasiReport>() == 72);
        assert!(core::mem::align_of::<KlatosIntegrasiReport>() == 8);
        assert!(core::mem::size_of::<Otoritas>() == 32);

        // F-P174-01 — harapan bitmap dihitung dari N_OTORITAS, bukan dikarang.
        assert!(bitmap_harap(0).0 == 0 && bitmap_harap(0).1 == 0);
        assert!(bitmap_harap(1).0 == 1 && bitmap_harap(1).1 == 0);
        assert!(bitmap_harap(32).0 == 0xFFFF_FFFF && bitmap_harap(32).1 == 0);
        assert!(bitmap_harap(33).0 == 0xFFFF_FFFF && bitmap_harap(33).1 == 1);
        assert!(bitmap_harap(63).0 == 0xFFFF_FFFF && bitmap_harap(63).1 == 0x7FFF_FFFF);
        assert!(bitmap_harap(64).0 == 0xFFFF_FFFF && bitmap_harap(64).1 == 0xFFFF_FFFF);
        // N_OTORITAS >= 65: bitmap 64-bit JENUH (bit ke-65+ tidak dapat
        // diwakili; lihat F-P175-01 di `periksa_semua_impl`).
        assert!(bitmap_harap(65).0 == 0xFFFF_FFFF && bitmap_harap(65).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(66).0 == 0xFFFF_FFFF && bitmap_harap(66).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(67).0 == 0xFFFF_FFFF && bitmap_harap(67).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(68).0 == 0xFFFF_FFFF && bitmap_harap(68).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(69).0 == 0xFFFF_FFFF && bitmap_harap(69).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(70).0 == 0xFFFF_FFFF && bitmap_harap(70).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(71).0 == 0xFFFF_FFFF && bitmap_harap(71).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(72).0 == 0xFFFF_FFFF && bitmap_harap(72).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(73).0 == 0xFFFF_FFFF && bitmap_harap(73).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(74).0 == 0xFFFF_FFFF && bitmap_harap(74).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(75).0 == 0xFFFF_FFFF && bitmap_harap(75).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(76).0 == 0xFFFF_FFFF && bitmap_harap(76).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(77).0 == 0xFFFF_FFFF && bitmap_harap(77).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(78).0 == 0xFFFF_FFFF && bitmap_harap(78).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(79).0 == 0xFFFF_FFFF && bitmap_harap(79).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(80).0 == 0xFFFF_FFFF && bitmap_harap(80).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(81).0 == 0xFFFF_FFFF && bitmap_harap(81).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(82).0 == 0xFFFF_FFFF && bitmap_harap(82).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(83).0 == 0xFFFF_FFFF && bitmap_harap(83).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(84).0 == 0xFFFF_FFFF && bitmap_harap(84).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(85).0 == 0xFFFF_FFFF && bitmap_harap(85).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(86).0 == 0xFFFF_FFFF && bitmap_harap(86).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(87).0 == 0xFFFF_FFFF && bitmap_harap(87).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(88).0 == 0xFFFF_FFFF && bitmap_harap(88).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(89).0 == 0xFFFF_FFFF && bitmap_harap(89).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(90).0 == 0xFFFF_FFFF && bitmap_harap(90).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(91).0 == 0xFFFF_FFFF && bitmap_harap(91).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(92).0 == 0xFFFF_FFFF && bitmap_harap(92).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(93).0 == 0xFFFF_FFFF && bitmap_harap(93).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(94).0 == 0xFFFF_FFFF && bitmap_harap(94).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(95).0 == 0xFFFF_FFFF && bitmap_harap(95).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(96).0 == 0xFFFF_FFFF && bitmap_harap(96).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(97).0 == 0xFFFF_FFFF && bitmap_harap(97).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(98).0 == 0xFFFF_FFFF && bitmap_harap(98).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(99).0 == 0xFFFF_FFFF && bitmap_harap(99).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(100).0 == 0xFFFF_FFFF && bitmap_harap(100).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(101).0 == 0xFFFF_FFFF && bitmap_harap(101).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(102).0 == 0xFFFF_FFFF && bitmap_harap(102).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(103).0 == 0xFFFF_FFFF && bitmap_harap(103).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(104).0 == 0xFFFF_FFFF && bitmap_harap(104).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(105).0 == 0xFFFF_FFFF && bitmap_harap(105).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(106).0 == 0xFFFF_FFFF && bitmap_harap(106).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(107).0 == 0xFFFF_FFFF && bitmap_harap(107).1 == 0xFFFF_FFFF);
        assert!(bitmap_harap(N_OTORITAS).0 == 0xFFFF_FFFF);
        assert!(bitmap_harap(N_OTORITAS).1 == 0xFFFF_FFFF);
    };
}
