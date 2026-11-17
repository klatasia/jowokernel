// Copyright (c) 2025 WSM SOFTWARE.
// SPDX-License-Identifier: MIT

//! KlatKernel — prototipe **inti kernel Rust** KLATOS (P0.97).

#![no_std]
// Tidak ada blok `unsafe` di luar `arch`, `ffi`, dan `frame`. `deny` (bukan
// `forbid`) dipakai karena rustc mengklasifikasi *atribut* `#[unsafe(no_mangle)]`
// sebagai `unsafe_code`; ketiga modul itu membuka izinnya sendiri secara
// bertarget, dan setiap blok `unsafe` di dalamnya diberi komentar SAFETY.
//
// `frame` (P0.99) membuka izin karena ia adalah OTORITAS FRAME FISIK: ia harus
// menafsirkan bitmap dan tautan freelist yang disediakan C++ sebagai media
// penyimpanan. Blok `unsafe`-nya kecil, terkurung pada akses memori mentah, dan
// jumlahnya dilaporkan apa adanya di laporan gate (§21).
#![deny(unsafe_code)]

// Kontrak ABI v2 + inti dispatch P0.74, disertakan verbatim lewat `#[path]`.
#[path = "../../Semesta/Fondasi/KlatIpc/src/abi2.rs"]
pub mod abi2;
#[path = "../../Semesta/Fondasi/KlatIpc/src/dispatch.rs"]
pub mod dispatch;
// FASE-7 (B1) — pintu masuk ABI v2 Design B (`entry.rs`), disertakan verbatim
// dari crate kontrak. Sisi klien (`klien-ipc`) digerbangi fitur yang TIDAK
// dinyalakan di crate kernel, sehingga hanya codec + handler murni yang masuk.
// Ini menutup celah FASE-6: simbol `klatos_abi2_entry_*` sebelumnya hanya ada di
// `KlatIpcKernel` (crate yang TIDAK ditautkan); `KlatKernel` adalah arsip yang
// benar-benar ada di baris tautan kernel.
#[path = "../../Semesta/Fondasi/KlatIpc/src/entry.rs"]
pub mod entry;
// FASE-7 (B2) — MEKANISME operasi ABI v2 (§7.5). Menggantikan `Ok(token)` pada
// jalur kernel: setiap operasi VFS/FS/input menghasilkan nilai nyata atau galat
// pasti, memakai otoritas Rust yang sudah ada.
#[allow(unsafe_code)]
pub mod mekanisme_abi2;

#[allow(unsafe_code)]
pub mod arch;
pub mod boot;
pub mod ffi;
// P0.99 — otoritas frame fisik produksi. Satu-satunya modul yang boleh menulis
// bitmap/freelist zona; lihat dokumentasi modulnya.
#[allow(unsafe_code)]
pub mod frame;
pub mod interrupt;
pub mod ipc;
pub mod ksm;
pub mod memory;
pub mod memtask;
pub mod panic;
pub mod scheduler;
pub mod serial;
pub mod task;
// P1.00 — otoritas identitas task (PID/TID dalam satu ruang id). Menggantikan
// `next_pid`/`Process::allocate_pid()` di C++.
//
// `allow(unsafe_code)` di sini membuka izin HANYA karena rustc
// mengklasifikasi *atribut* `#[no_mangle]` sebagai `unsafe_code` (alasan yang
// sama dengan `arch`, `ffi`, dan `frame`). Modul ini sendiri **tidak memuat
// satu blok `unsafe` pun** — dapat diperiksa: `grep -c 'unsafe {' task_id.rs`
// harus 0. Ia tidak menafsirkan memori mentah apa pun; hanya atomik.
#[allow(unsafe_code)]
pub mod task_id;
// P1.01 — otoritas keputusan ember Scheduler (priority bucket). Menggantikan
// `thread_priority_to_priority_index` di `KlatKernel/Efek/Eksekusi/Scheduler.cpp:59-69`.
// Keputusan: prioritas integer mana -> ember `0..=31`. Dipanggil dari
// `Scheduler::enqueue_runnable_thread` (Scheduler.cpp:171).
//
// `allow(unsafe_code)` dengan alasan yang sama dengan `task_id`. Modul ini
// murni (tidak ada atomik di jalur keputusan) dan `grep -c 'unsafe'
// priority_bucket.rs` di baris kode (di luar komentar) harus 0.
#[allow(unsafe_code)]
pub mod priority_bucket;
// P1.02 — otoritas keputusan validasi handle IPC Jowo. Menggantikan
// `JowoIPCHandles::get(int)` di `KlatKernel/Efek/Eksekusi/Process.h:1033-1040`.
// Keputusan: handle->`Ok(idx)` (>=0) | `Err(EINVAL)` (<0) | `Err(EBADF)`
// (<0). Dipanggil dari setiap syscall IPC (`klatipc_send`/`klatipc_recv`/
// `klatipc_close`/`klatipc_give`/`klatipc_accept`) di `KlatIpc/Kanal/klatipc.cpp`.
//
// `allow(unsafe_code)` dengan alasan yang sama dengan modul-modul di atas.
// Modul ini murni (`grep -c '\<unsafe\>'` di luar komentar harus 0).
#[allow(unsafe_code)]
pub mod ipc_handle;
// P1.03 — otoritas keputusan validasi clock_id Jowo. Menggantikan
// `TimeManagement::validate_clock_id(clockid_t)` di
// `KlatKernel/Efek/Inti/TimeManagement.cpp:84-95`. Keputusan: clock_id salah satu dari
// 5 nilai sah (`CLOCK_REALTIME=0`, `CLOCK_MONOTONIC=1`, `CLOCK_MONOTONIC_RAW=2`,
// `CLOCK_REALTIME_COARSE=3`, `CLOCK_MONOTONIC_COARSE=4`) -> `Ok(c)` (>=0),
// selainnya -> `Err(EINVAL)` (<0). Dipanggil dari tiga syscall waktu
// (`sys$clock_gettime` line 34, `sys$clock_nanosleep` line 84,
// `sys$clock_getres` line 109) di `Jowo/Panggilan/clock.cpp`.
//
// `allow(unsafe_code)` dengan alasan yang sama dengan modul-modul di atas.
// Modul ini murni (`grep -c '\<unsafe\>'` di luar komentar harus 0).
#[allow(unsafe_code)]
pub mod clock_id;
// P1.04 — BATAS FPU/SSE UNTUK JALUR TRAP. Menutup F-P099-07 / F-P100-09:
// pintu masuk trap (`ISR stub`, `common_trap_exit`, `syscall_entry`) TIDAK
// menyimpan state FPU, sedangkan Rust wajib SSE2 dan rustc menolak
// mematikannya (F-P099-06). Modul ini menyediakan penjaga `fxsave64`/
// `fxrstor64` di BATAS trap→Rust sehingga jaminannya arsitektural, bukan
// kebetulan codegen.
//
// `allow(unsafe_code)` di sini bukan formalitas: modul ini MEMANG memuat
// `unsafe` — ia mengeksekusi instruksi XMM/FXSAVE langsung. Itu satu-satunya
// alasan ia ada. Modul ini TIDAK memiliki state FPU thread; C++ tetap pemilik
// `Thread::m_fpu_state` (`Processor::store_fpu_state`/`load_fpu_state`).
#[allow(unsafe_code)]
pub mod fpu;
// P1.05 — OTORITAS DISPATCH INTERUPSI (MODEL B). Menggantikan keputusan
// pemilihan handler di `handle_interrupt(TrapFrame*)`
// (`Jowo/Arsitektur/x86_64/Interrupts.cpp:304-330`), satu-satunya pintu IRQ
// kernel. Keputusan: vektor `isr` -> kelas (generik / PIC-disabled / ditolak),
// IRQ di dalam kelas, indeks slot, tabel mana yang dipakai, dan apakah EOI
// diminta. Dipanggil dari `handle_interrupt` di dalam penjaga FPU P1.04.
//
// MODEL B: IDT, `lidt`, stub ASM, `common_trap_exit`, penyimpanan tabel handler,
// pemanggilan objek handler, dan kebijakan PENDAFTARAN tetap milik C++. Rust
// tidak pernah memasang IDT — `klatos_int_owns_idt()` selalu 0.
//
// `allow(unsafe_code)` dengan alasan yang sama dengan `clock_id`/`task_id`:
// rustc mengklasifikasi atribut `#[no_mangle]` sebagai `unsafe_code`. Modul ini
// sendiri TIDAK memuat satu blok `unsafe` pun — dapat diperiksa:
// `grep -c 'unsafe {' int.rs` harus 0. Ia hanya memakai atomik dan aritmetika.
#[allow(unsafe_code)]
pub mod int;
// P1.06 — OTORITAS TICK TIMER PRODUKSI (MODEL B). Menggantikan keputusan
// kebijakan tick di `TimeManagement::system_timer_callback()`
// (`KlatKernel/Efek/Inti/TimeManagement.cpp:507-522`) dan keputusan pengendali EOI di
// `HardwareTimer<GenericInterruptHandler>::eoi()`
// (`Jowo/Arsitektur/x86_64/Interrupts/APIC.cpp:710-714`).
//
// Timer yang BENAR-BENAR berjalan pada sistem ini adalah **Local APIC timer**
// (log boot: `Duration: Using APIC timer as system timer`,
// `APICTimer: Ticks per second: 250`); HPET hanya sumber kalibrasi. Vektor
// timer = `0xfc`, IRQ = `0xac`.
//
// Keputusan: untuk setiap tick pada vektor timer -> kelas CPU (BSP/AP),
// `perbarui_waktu`, `bakar_antrean`, `jadwalkan`, `eoi_diminta`; dan untuk EOI
// objek timer -> `perlu_eoi` + pengendali (`APIC`/`PIC`/tidak).
//
// MODEL B: IDT, `lidt`, stub ASM, `common_trap_exit`, objek timer, pemrograman
// sumber timer, dan isi `update_time()`/`TimerQueue::fire()`/
// `Scheduler::timer_tick()`/`APIC::the().eoi()` tetap milik C++.
//
// Aritmetika tick (`increment_time_since_boot`) SENGAJA TIDAK dipindahkan: ia
// terukur MATI pada runtime karena jalur kueri presisi (HPET/TSC) aktif, dan
// memindahkannya akan menciptakan penghitung tick KEDUA — otoritas kedua atas
// `m_ticks_this_second`. Lihat dokumentasi modulnya.
//
// `allow(unsafe_code)` dengan alasan yang sama dengan `int`/`clock_id`: rustc
// mengklasifikasi atribut `#[no_mangle]` sebagai `unsafe_code`. Modul ini
// sendiri TIDAK memuat satu blok `unsafe` pun — dapat diperiksa:
// `grep -c 'unsafe {' timer.rs` harus 0.
#[allow(unsafe_code)]
pub mod timer;

/// P1.09: **otoritas kebijakan proteksi mmap W^X produksi** — apakah sebuah
/// permintaan `mmap`/`mprotect` boleh membuat satu region dapat-ditulis
/// sekaligus dapat-dieksekusi. Sebelum P1.09 rumus enam pemeriksaan itu
/// milik `Process::validate_mmap_prot` (`KlatKernel/Efek/Memori/mmap.cpp:62-88`),
/// dipanggil dari `sys$mmap` (`mmap.cpp:166`) dan `sys$mprotect`
/// (`mmap.cpp:276`/`:295`).
///
/// C++ hanya mengumpulkan input (membaca `mount_flags` dan riwayat `Region`)
/// lalu memanggil `klatos_mmap_prot_decide`; rumusnya tidak lagi ada di C++.
/// Keputusan dijalankan di dalam penjaga FPU (`dengan_penjaga_fpu_rust`)
/// karena `sys$mmap`/`sys$mprotect` berada di jalur trap (F-P099-07).
///
/// Tidak ada syscall/ioctl/opcode/token baru, tidak ada kapasitas KSM baru.
/// `allow(unsafe_code)` dengan alasan yang sama dengan `timer`: rustc
/// mengklasifikasi `#[no_mangle]` sebagai `unsafe_code`; modul ini sendiri
/// TIDAK memuat satu blok `unsafe` pun (`grep -c 'unsafe {' mmap_prot.rs`
/// harus 0).
#[allow(unsafe_code)]
pub mod mmap_prot;

/// P1.10: **otoritas pledge mode-akses `open()` produksi** — himpunan pledge
/// (`wpath`/`rpath`/`cpath`) yang wajib dimiliki pemanggil untuk sebuah
/// `open`. Sebelum P1.10 rumus `options -> pledge` milik
/// `Process::open_impl` (`Jowo/Panggilan/open.cpp:45-53`), dipanggil dari
/// `sys$open`/`sys$openat`. C++ hanya menjalankan `require_promise` untuk
/// setiap bit yang dikembalikan Rust.
#[allow(unsafe_code)]
pub mod ksm_open;

/// P1.11: **otoritas mode akses baca/tulis deskriptor berkas produksi** —
/// sebuah MEKANISME: dua boolean yang disimpan di state deskriptor dan
/// menjadi satu-satunya gerbang setiap `read()`/`write()` (`EBADF`).
/// Sebelum P1.11 rumus `options -> (readable, writable)` milik
/// `OpenFileDescription::set_rw_mode`
/// (`KlatKernel/Efek/Berkas/OpenFileDescription.cpp:510`), dipanggil dari lima jalur
/// `open` (`VirtualFileSystem`/`File`/`SelfTTYDevice`/`PTYMultiplexer`).
#[allow(unsafe_code)]
pub mod fd_mode;

/// P1.12: **otoritas aksi default sinyal produksi** — pemetaan 32 sinyal ke
/// lima aksi (`Terminate`/`Ignore`/`DumpCore`/`Stop`/`Continue`). Sebelum
/// P1.12 `switch` itu milik `default_signal_action`
/// (`KlatKernel/Efek/Eksekusi/Thread.cpp:804-845`), dipanggil dari
/// `Thread::should_ignore_signal` dan jalur pengiriman sinyal.
#[allow(unsafe_code)]
pub mod signal_action;

/// P1.13: **otoritas penguraian string izin `unveil()` produksi** — lima
/// karakter (`r/w/x/c/b`) ke lima bit `IzinAksesJalur`. Sebelum P1.13 loop itu
/// milik `Process::sys$unveil` (`Jowo/Keamanan/akses_jalur.cpp:80-99`). String
/// dipadatkan C++ ke `u64` (<= 5 byte, `get_syscall_string_fixed_buffer<5>`),
/// sehingga modul ini tidak memuat blok `unsafe` — sama seperti P1.09–P1.12.
#[allow(unsafe_code)]
pub mod ksm_parser_izin;

/// P1.14: **otoritas admisi permintaan `unveil()` produksi** — gerbang
/// `sys$unveil` yang memutuskan `Lock` / `Ignore` / `EPERM` / `EINVAL` /
/// `Proses` dari flag, kelayakan jalur, dan keadaan veil. Sebelum P1.14 blok
/// itu milik `Process::sys$unveil` (`Jowo/Keamanan/akses_jalur.cpp:86-110`).
/// Keputusan dua fase (`ADM_BUTUH_PATH`) supaya penyalinan jalur dari ruang
/// pengguna terjadi pada urutan yang sama dengan C++ historis; hanya integer
/// yang menyeberang batas sehingga modul ini tidak memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod ksm_keputusan_jalur;

/// P1.15: **otoritas transisi keadaan janji `pledge()` produksi** — himpunan
/// bagian / penyempitan / penolakan `EPERM` atas bitmask janji. Sebelum P1.15
/// rumus itu milik `Process::sys$pledge` (`Jowo/Keamanan/ksm_keamanan.cpp:63-96`).
/// Berbeda dari P1.10 (mode akses `open()` -> janji yang dibutuhkan): modul ini
/// memindahkan transisi TABEL janji itu sendiri. Hanya integer yang menyeberang
/// batas sehingga modul ini tidak memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod ksm_transisi;

/// P1.16: **otoritas legalitas + klasifikasi aksi transisi keadaan `Thread`
/// produksi** — pasangan `(previous_state, new_state)` mana yang sah, pesan
/// PANIC yang benar, dan himpunan aksi mekanisme yang wajib dijalankan.
/// Sebelum P1.16 keputusan itu milik `Thread::set_state`
/// (`KlatKernel/Efek/Eksekusi/Thread.cpp:1207-1263`). Penjadwal itu sendiri (`pick_next`,
/// `context_switch`, run queue, time slice) TIDAK disentuh. Dua `u32` masuk dan
/// satu struct 8 byte keluar; tidak ada pointer sehingga modul ini tidak memuat
/// blok `unsafe`.
#[allow(unsafe_code)]
pub mod thread_transisi;

/// P1.17: **otoritas kebijakan `seek()` deskriptor berkas produksi** — offset
/// baru atau errno (`ESPIPE`/`EINVAL`/`EOVERFLOW`/`EIO`) untuk `lseek(2)`.
/// Sebelum P1.17 keputusan itu milik `OpenFileDescription::seek`
/// (`KlatKernel/Efek/Berkas/OpenFileDescription.cpp:110-142`). Efek samping (`did_seek`,
/// `evaluate_block_conditions`) TIDAK disentuh. Lima integer masuk dan satu
/// `i64` keluar; tidak ada pointer sehingga modul ini tidak memuat blok
/// `unsafe`.
#[allow(unsafe_code)]
pub mod seek_kebijakan;

/// P1.18: **otoritas dekode/normalisasi operasi `futex()` produksi** —
/// pemisahan `FUTEX_CMD_MASK`/`FUTEX_CLOCK_REALTIME`/`FUTEX_PRIVATE_FLAG`,
/// penolakan realtime pada non-wait (`ENOSYS`), normalisasi `*_BITSET` +
/// `FUTEX_BITSET_MATCH_ANY`, penolakan bitset kosong (`EINVAL`), dan operasi
/// tak dikenal (`ENOSYS`). Sebelum P1.18 dekode itu milik `Process::sys$futex`
/// (`KlatKernel/Efek/Eksekusi/futex.cpp:99-128` + `:331`). Mekanisme antrean (`do_wait`,
/// `do_wake`, `do_requeue`, `FutexQueue`) TIDAK disentuh. Dua `u32` masuk dan
/// satu struct 12 byte keluar; tidak ada pointer sehingga modul ini tidak
/// memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod futex_op;

/// P1.19: **otoritas validasi alamat pengguna produksi** — apakah sebuah
/// `VirtualAddress` berada di rentang ruang pengguna. Sebelum P1.19 rumusnya
/// milik `Jowo::Memory::is_user_address` (`KlatKernel/Efek/Memori/MemorySections.h:18-21`),
/// yaitu `vaddr.get() < USER_RANGE_CEILING` dengan
/// `USER_RANGE_CEILING = g_boot_info.kernel_mapping_base - 0x2000000`
/// (`KlatKernel/Efek/Inti/Sections.h:33`). C++ hanya meneruskan `vaddr.get()` dan
/// `g_boot_info.kernel_mapping_base`; aritmetika pita pengaman 32 MiB dan
/// perbandingannya kini milik Rust. Dipanggil dari jalur syscall/trap
/// (`UserOrKernelBuffer`, `MemoryManager`, `Region`, `Thread`,
/// `PerformanceEventBuffer`, `Processor`), sehingga pemanggil membungkusnya
/// `dengan_penjaga_fpu_rust` (F-P099-07). Dua `u64` masuk, satu `u32` keluar;
/// tidak ada pointer sehingga modul ini tidak memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod alamat_pengguna;

/// P1.20: **otoritas validasi rentang pengguna produksi** — apakah
/// `[vaddr, vaddr+size)` seluruhnya berada di ruang pengguna, dengan cek
/// overflow. Sebelum P1.20 rumusnya milik `Jowo::Memory::is_user_range`
/// (`KlatKernel/Efek/Memori/MemorySections.h:23-32`). Modul ini MEMAKAI otoritas P1.19
/// ([`alamat_pengguna::adalah_alamat_pengguna`]) sehingga hanya ada satu
/// salinan rumus alamat. Dipanggil dari jalur syscall/trap
/// (`StdLib` copy_to/from_user, `UserOrKernelBuffer`, `mmap`, `futex`,
/// `Thread`, `sigaction`), sehingga pemanggil membungkusnya
/// `dengan_penjaga_fpu_rust`. Tiga `u64` masuk, satu `u32` keluar; tidak ada
/// pointer sehingga modul ini tidak memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod rentang_pengguna;

/// P1.21: **otoritas predikat keanggotaan `VirtualRange` produksi** — apakah
/// sebuah alamat (`contains(VirtualAddress)`) atau sub-rentang
/// (`contains(VirtualAddress, size_t)`) berada di dalam sebuah rentang.
/// Sebelum P1.21 kedua rumus itu milik `KlatKernel/Efek/Memori/VirtualRange.h:34` dan
/// `:42-47`. Diferensial menangkap asimetri ujung: alamat memakai ujung
/// EKSKLUSIF, sub-rentang memakai ujung INKLUSIF. Dipanggil dari `Region::contains`
/// (jalur syscall/trap) dan `RegionTree`/`kmalloc`. Empat `u64` masuk, satu
/// `u32` keluar; tidak ada pointer sehingga modul ini tidak memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod rentang_berisi;

/// P1.22: **otoritas klasifikasi ruang wilayah produksi** — apakah alamat
/// sebuah `Region` berada di wilayah kernel (`is_kernel`) atau pengguna
/// (`is_user`). Sebelum P1.22 rumusnya milik `KlatKernel/Efek/Memori/Region.h:105-106`,
/// yaitu `vaddr < USER_RANGE_BASE || vaddr >= kernel_mapping_base`. BERBEDA dari
/// P1.19: dua batas dan `kernel_mapping_base` LANGSUNG (bukan plafon berpita
/// pengaman). Dipanggil dari `Coredump`, `MemoryManager`, `ProcessExposed`.
/// Tiga `u64` masuk, satu `u32` keluar; tidak ada pointer sehingga modul ini
/// tidak memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod wilayah_ruang;

/// P1.23: **otoritas predikat tumpang-tindih `VirtualRange` produksi** — apakah
/// dua rentang bertumpang tindih. Sebelum P1.23 rumusnya milik
/// `VirtualRange::intersects` (`KlatKernel/Efek/Memori/VirtualRange.cpp:26-35`). Diferensial
/// menangkap ketaksamaan TEGAS: rentang bersebelahan dan rentang kosong TIDAK
/// bertumpang. Dipanggil dari `RegionTree::allocate_range_specific`. Empat `u64`
/// masuk, satu `u32` keluar; tidak ada pointer sehingga modul ini tidak memuat
/// blok `unsafe`.
#[allow(unsafe_code)]
pub mod rentang_tumpang;

/// P1.24: **otoritas validasi `signum` sigaction produksi** — apakah
/// `signum` masuk rentang `[1..NSIG)` dan bukan `SIGKILL`/`SIGSTOP`. Sebelum
/// P1.24 satu baris itu milik `Process::sys$sigaction`
/// (`KlatKernel/Efek/Eksekusi/sigaction.cpp:58`). Dipanggil dari jalur syscall (syscall
/// 129), sehingga pemanggil membungkusnya `dengan_penjaga_fpu_rust`
/// (F-P099-07). Satu `i32` masuk, satu `i32` keluar; tidak ada pointer
/// sehingga modul ini tidak memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod validasi_nomor_sinyal;

/// P1.25: **otoritas validasi kombinasi bendera `mmap` produksi** — empat
/// pemeriksaan kombinasi (`shared XOR private`, `fixed + randomized`,
/// `stack` butuh `private + anonymous`). Sebelum P1.25 keempat baris itu
/// milik `Process::sys$mmap` (`KlatKernel/Efek/Memori/mmap.cpp:163-175`). Baris
/// `validate_mmap_prot` adalah wewenang P1.09 (`mmap_prot`) dan TIDAK
/// disentuh. Dipanggil dari jalur syscall (syscall 9), sehingga pemanggil
/// membungkusnya `dengan_penjaga_fpu_rust`. Satu `u32` masuk, satu `i32`
/// keluar; tidak ada pointer sehingga modul ini tidak memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod kebijakan_bendera_mmap;

/// P1.26: **otoritas validasi `cmd` `fcntl` produksi** — `cmd` salah satu
/// dari 10 nilai (`F_DUPFD=0`..`F_DUPFD_CLOEXEC=9`) atau `-EINVAL`.Sebelum
/// P1.26 `default` pada `switch` itu milik `Process::sys$fcntl`
/// (`Jowo/Panggilan/fcntl.cpp:16-60`); hanya keputusan "cmd dikenal"
/// yang dipindahkan — cabang-cabang switch tetap di C++. Dipanggil dari
/// jalur syscall (syscall 72), sehingga pemanggil membungkusnya
/// `dengan_penjaga_fpu_rust`. Satu `i32` masuk, satu `i32` keluar; tidak
/// ada pointer sehingga modul ini tidak memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod validasi_perintah_fcntl;

/// P1.27: **otoritas validasi topeng `flags` `faccessat` produksi** — bit
/// `flags` di luar `AT_SYMLINK_NOFOLLOW|AT_EACCESS` ditolak. Sebelum P1.27
/// baris itu milik `Process::sys$faccessat`
/// (`Jowo/Panggilan/faccessat.cpp:23`). Dipanggil dari jalur syscall
/// (syscall 269), sehingga pemanggil membungkusnya
/// `dengan_penjaga_fpu_rust`. Satu `i32` masuk, satu `i32` keluar; tidak
/// ada pointer sehingga modul ini tidak memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod validasi_bendera_faccessat;

/// P1.28: **otoritas penolakan bendera terlarang `open()` produksi** —
/// `O_NOFOLLOW_NOERROR` (`1<<29`) dan `O_UNLINK_INTERNAL` (`1<<30`) dari
/// program pengguna selalu `-EINVAL`. Sebelum P1.28 kedua baris itu milik
/// `Process::open_impl` (`Jowo/Panggilan/open.cpp:30-34`). Dipanggil dari
/// jalur syscall (`sys$open`/`sys$openat`), sehingga pemanggil
/// membungkusnya `dengan_penjaga_fpu_rust`. Satu `i32` masuk, satu `i32`
/// keluar; tidak ada pointer sehingga modul ini tidak memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod validasi_bendera_open_larangan;

/// P1.29: **otoritas izin akses berkas DAC Unix produksi** — pintasan root,
/// pemilihan bit izin pemilik/grup/lainnya. Sebelum P1.29 formula itu milik
/// `InodeMetadata::may_read/may_write/may_execute`
/// (`KlatKernel/Efek/Berkas/InodeMetadata.h:54-85`). Uji keanggotaan `extra_gids`
/// (span) tetap di C++ dan diteruskan sebagai `egid_match`; keputusan
/// pemilihan bit milik Rust. Dipanggil dari jalur syscall VFS, sehingga
/// pemanggil membungkusnya `dengan_penjaga_fpu_rust`. Tujuh skalar masuk,
/// satu `i32` keluar; tidak ada pointer sehingga modul ini tidak memuat blok
/// `unsafe`.
#[allow(unsafe_code)]
pub mod izin_inode;

/// P1.30: **otoritas izin pengiriman sinyal produksi** — disjungsi empat suku
/// (root / euid cocok / uid cocok / SIGCONT+pgid cocok). Sebelum P1.30 formula
/// itu milik `Process::do_kill` (`KlatKernel/Efek/Eksekusi/kill.cpp:23-27`). Tujuh skalar
/// masuk, satu `i32` keluar; tidak ada pointer sehingga modul ini tidak
/// memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod izin_sinyal;

/// P1.31: **otoritas izin transisi identitas produksi** — delapan aturan
/// setuid/setgid/seteuid/setegid/setreuid/setresuid/setregid/setresgid.
/// Sebelum P1.31 kedelapan aturan itu milik `KlatKernel/Efek/Eksekusi/setuid.cpp`.
/// Sebelas skalar masuk, satu `i32` keluar; tidak ada pointer sehingga modul
/// ini tidak memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod izin_identitas;

/// P1.32: **otoritas izin parameter penjadwal produksi** — batas prioritas
/// `[THREAD_PRIORITY_MIN, MAX]` untuk `set` dan cek izin pemilik/root. Sebelum
/// P1.32 formula itu milik `KlatKernel/Efek/Eksekusi/sched.cpp:74-79,105-107`. Tujuh
/// skalar masuk, satu `i32` keluar; tidak ada pointer sehingga modul ini tidak
/// memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod izin_jadwal;

/// P1.33: **otoritas penerimaan bendera mount produksi** — `MS_REMOUNT` dan
/// `MS_BIND` selalu `-EINVAL`. Sebelum P1.33 baris itu milik empat syscall
/// mount di `Jowo/Panggilan/mount.cpp`. Satu `i32` masuk, satu `i32` keluar;
/// tidak ada pointer sehingga modul ini tidak memuat blok `unsafe`.
#[allow(unsafe_code)]
pub mod mount_bendera;

/// P1.34–P1.36: **INFRASTRUKTUR RUANG-ID RUST** — satu algoritma penghitung
/// id monoton tanpa kunci ([`ruang_id::ambil_in`]) yang dipakai bersama tiga
/// gate kepemilikan identitas. Modul ini **tidak** punya permukaan C ABI dan
/// **tidak** memuat blok `unsafe` (hanya atomik + `Cell`).
pub mod ruang_id;

/// P1.34: **otoritas identitas objek VFS produksi** — kepemilikan tiga
/// penghitung id yang sebelumnya `static` C++: `FileSystemID`
/// (`KlatKernel/Efek/Berkas/FileSystem.cpp:22,25`), `VFSRootContext::IndexID`
/// (`KlatKernel/Efek/Berkas/VFSRootContext.cpp:20,80`), dan `FIFO::m_fifo_id`
/// (`KlatKernel/Efek/Berkas/FIFO.cpp:20,71`). Sejak P1.34 C++ tidak lagi memiliki
/// penghitung itu. `allow(unsafe_code)` hanya karena rustc mengklasifikasi
/// atribut `#[no_mangle]` sebagai `unsafe_code`; modul ini sendiri tidak
/// memuat satu blok `unsafe` pun.
#[allow(unsafe_code)]
pub mod id_vfs;

/// P1.35: **otoritas penomoran inode produksi** — indeks komponen SysFS
/// GLOBAL (`KlatKernel/Efek/Berkas/SysFS/Component.cpp:20-27`) dan indeks inode RAMFS
/// PER-INSTANCE (`KlatKernel/Efek/Berkas/RAMFS/FileSystem.h:45`, `.cpp:57`). Yang kedua
/// dipindahkan sebagai **handle integer** (Rust memegang tabel ruang),
/// sehingga tidak ada pointer C++ yang menyeberangi FFI. `allow(unsafe_code)`
/// dengan alasan yang sama dengan `id_vfs`; tidak ada blok `unsafe`.
#[allow(unsafe_code)]
pub mod id_inode;

/// P1.36: **otoritas identitas objek namespace proses produksi** —
/// `HostnameContext::IndexID` (`KlatKernel/Efek/Eksekusi/HostnameContext.cpp:17,96`) dan
/// `ScopedProcessList::IndexID` (`KlatKernel/Efek/Eksekusi/ScopedProcessList.cpp:16,55`).
/// `allow(unsafe_code)` dengan alasan yang sama dengan `id_vfs`; tidak ada
/// blok `unsafe`.
#[allow(unsafe_code)]
pub mod id_proses;

/// P1.37: **otoritas transisi keadaan PRNG cepat produksi** — langkah LCG
/// `x' = 1103515245·x + 12345 (mod 2^32)` yang sebelumnya ada di
/// `do_get_fast_random_bytes` (`Jowo/Keamanan/Random.cpp:150-166`). Keadaan
/// generator kini milik Rust; C++ mempertahankan perulangan pengisian buffer
/// (ia menerima pointer `Bytes`, yang dilarang menyeberangi FFI).
/// `allow(unsafe_code)` dengan alasan yang sama; tidak ada blok `unsafe`.
#[allow(unsafe_code)]
pub mod acak_cepat;

/// P1.38: **otoritas akuntansi waktu CPU produksi** — pasangan
/// `TotalTimeScheduled{total, total_kernel}` yang sebelumnya dijaga
/// `RecursiveSpinlockProtected` (`KlatKernel/Efek/Eksekusi/Scheduler.cpp:56,442-449`).
/// Kini dua [`core::sync::atomic::AtomicU64`]; lihat dokumentasi modul untuk
/// perbedaan snapshot yang disengaja. `allow(unsafe_code)` dengan alasan yang
/// sama; tidak ada blok `unsafe`.
#[allow(unsafe_code)]
pub mod akuntansi_waktu;

/// P1.39: **otoritas antrean peristiwa perangkat produksi** — seluruh isi
/// `SpinlockProtected<CircularQueue<DeviceEvent, 100>, LockRank::None>`
/// (`Jowo/Perangkat/Device.cpp:26`) kini milik Rust: `head`, `size`, 100 slot,
/// dan transisi masuk/keluar/luapan. C++ hanya memanggil batas tipis ini dari
/// `Device.cpp` (produsen) dan `DeviceControlDevice.cpp` (konsumen).
/// `allow(unsafe_code)` dengan alasan yang sama; tidak ada blok `unsafe`.
#[allow(unsafe_code)]
pub mod antrean_perangkat;

/// P1.40: **otoritas metadata penyangga ganda produksi** — seluruh state
/// machine `Jowo::DoubleBuffer` (`m_write_buffer`/`m_read_buffer`,
/// `m_read_buffer_index`, `m_space_for_writing`, `m_empty`, `flip()`,
/// `compute_lockfree_metadata()`) kini milik Rust. C++ hanya menyimpan byte
/// (`KBuffer`) dan memanggil batas tipis ini dari `KlatKernel/Efek/Inti/DoubleBuffer.cpp`.
/// Penunjuk mentah tidak pernah menyeberangi FFI: Rust mengembalikan
/// `paruh` + `offset` sebagai [`penyangga_ganda::KlatKursor`].
/// `allow(unsafe_code)` dengan alasan yang sama; tidak ada blok `unsafe`.
#[allow(unsafe_code)]
pub mod penyangga_ganda;

/// P1.41: **otoritas siklus hidup pasangan PTY** — Rust owns the 64-entry
/// master-index freelist and per-index lifecycle bits; C++ retains only the
/// MasterPTY/SlavePTY objects and references.
#[allow(unsafe_code)]
pub mod siklus_pty;

/// P1.42: **otoritas registry perangkat** — Rust owns membership, duplicate
/// rejection, and removal lifecycle for `(kind, major, minor)` keys; C++ keeps
/// only an object index for method dispatch.
#[allow(unsafe_code)]
pub mod pendaftaran_perangkat;

/// P1.43: Rust-owned kernel clock accumulator and seqlock.
#[allow(unsafe_code)]
pub mod jam_inti;

/// P1.44: **otoritas buku besar bookkeeping IRQ produksi** — seluruh agregat
/// lintas-CPU atas hit/EOI IRQ (16 slot PIC klasik + spurious). Sebelum
/// P1.44 ketiga penghitung (`m_per_cpu_call_counts` + EOI count + is_shared)
/// tersebar di tiga tempat C++: `GenericInterruptHandler::increment_call_count`
/// (`Jowo/Sinkronisasi/GenericInterruptHandler.cpp:78`),
/// `IRQHandler::eoi()` (`Jowo/Sinkronisasi/IRQHandler.cpp:27`), dan
/// `SharedIRQHandler::handle_irq` (`Jowo/Sinkronisasi/SharedIRQHandler.cpp:81`).
/// Kini agregat lintas-CPU + buku besar 16 slot adalah otoritas Rust; C++ hanya
/// memanggil batas tipis ini. `allow(unsafe_code)` dengan alasan yang sama
/// dengan `jam_inti`; tidak ada blok `unsafe` (input skalar, atomik).
#[allow(unsafe_code)]
pub mod buku_besar_irq;

/// P1.45: **otoritas buku besar siklus hidup benang** — seluruh agregat
/// kelahiran/pemusnahan benang dan matriks masuk/keluar per `Thread::State`.
/// Sebelum P1.45 tidak ada satu tempat pun yang menjawab "berapa puncak benang
/// hidup?" atau "keadaan mana yang paling sering dimasuki?"; `dbgln_if` di
/// `KlatKernel/Efek/Eksekusi/Thread.cpp` hanya menyiarkan peristiwa tunggal.
/// **BUKAN** duplikat P1.16: legalitas + klasifikasi aksi transisi sudah milik
/// Rust sejak `thread_transisi.rs` (`klatos_thread_transisi_decide`); modul ini
/// hanya MENGHITUNG peristiwa yang sudah diputuskan. Penjadwal
/// (`pick_next`/`context_switch`/run queue/time slice) TIDAK disentuh.
/// `allow(unsafe_code)` dengan alasan yang sama dengan `buku_besar_irq`; tidak
/// ada blok `unsafe` (input skalar, atomik).
#[allow(unsafe_code)]
pub mod buku_besar_benang;

/// P1.46: **otoritas buku besar registry proses** — tabel slot berbatas
/// `(pid, ppid, kernel?)` + penghitung siklus hidup (dibuat/dilepas/hidup/
/// puncak/yatim/kernel/pengguna) + penghitungan anak langsung.
/// Sebelum P1.46 daftar proses hidup hanyalah `IntrusiveList` C++; tak ada
/// tabel berbatas yang menjawab "apakah pid N terdaftar?" atau "berapa anak
/// langsung pid P?".
/// **BUKAN** pohon proses pointer-heavy: `IntrusiveList<Process>`,
/// `m_process_list`, `RefPtr<Process>`, `for_each*`, `from_pid_*`,
/// `allocate_pid()` — semuanya tetap milik C++ dan TIDAK disentuh.
/// `allow(unsafe_code)` dengan alasan yang sama dengan `buku_besar_benang`;
/// tidak ada blok `unsafe` (input skalar, atomik + spinlock).
#[allow(unsafe_code)]
pub mod buku_besar_proses;

/// P1.47: **otoritas buku besar bookkeeping dispatch panggilan sistem** —
/// klasifikasi berbatas hasil dispatch (`OK`/`TAK_DIKENAL`/`ENOSYS`/
/// `PELANGGARAN`/`ERROR_LAIN`) + penghitung per-nomor-fungsi (tabel berbatas
/// `MAX_FUNGSI`). Sebelum P1.47 `Syscall::handle`
/// (`Jowo/Panggilan/SyscallHandler.cpp:42`) hanya mengembalikan
/// `ErrorOr<FlatPtr>`; tak ada klasifikasi berbatas maupun agregat.
/// **Nomor panggilan sistem TIDAK disentuh**: `enum Function`,
/// `ENUMERATE_SYSCALLS`, `s_syscall_table`, `SC_*`, `needs_lock` — semuanya
/// tetap C++ dan tidak berubah (ABI v1 FROZEN). Konstanta errno (`enosys`,
/// `epelanggaran`) DISUPLAI C++ karena errno adalah kontrak platform milik C++;
/// Rust yang memutuskan klasifikasinya.
/// `allow(unsafe_code)` dengan alasan yang sama; tidak ada blok `unsafe`.
#[allow(unsafe_code)]
pub mod buku_besar_panggilan;

/// P1.48 — **OTORITAS INTEGRASI INTI KERNEL RUST**.
///
/// Rust memiliki manifes kanonik 46 otoritas inti (id, nama, tag laporan yang
/// diharapkan) dan memanggil sendiri setiap `klatos_<otoritas>_selftest()`
/// lewat panggilan intra-crate, lalu memutuskan lulus/gagal. State `TAHAP`,
/// `SAH`, `TOLAK`, `TOLAK_PERTAMA`, `BITMAP`, `PAS` adalah milik Rust.
/// `boleh_lanjut()` adalah keputusan Rust; C++ hanya membacanya.
///
/// Sebelum P1.48, "apakah inti Rust terintegrasi?" hanya terjawab sebagai ~46
/// blok diagnostik terpisah di `Jowo/Arsitektur/init.cpp` — tanpa agregat,
/// tanpa manifes, dan tanpa keputusan. Manifes dibangkitkan dari sumber crate
/// ini, jadi ia tidak dapat menunjuk modul yang tidak ada (crate tidak akan
/// terkompilasi).
///
/// **Yang TIDAK dipindahkan**: isi setiap selftest otoritas, blok diagnostik
/// per-gate di `init.cpp`, dan apa pun yang berbau ABI v1/v2, syscall, KSM,
/// KlatIPC, atau Loader.
///
/// `allow(unsafe_code)` dengan alasan yang sama seperti modul lain; tidak ada
/// blok `unsafe` di badan modul.
#[allow(unsafe_code)]
pub mod integrasi_inti;

/// P1.49–P1.52: **OTORITAS GRAF OBJEK PROSES / BENANG**.
///
/// Sebelum gate ini keadaan logis proses dan benang hidup DI DALAM objek C++:
/// `Process::m_state`/`m_is_stopped`/`thread_count`/`termination_*`,
/// `Thread::m_state`/`m_is_joinable`/`m_is_active`/`m_should_die`/
/// `m_initialized`/`m_is_idle_thread`/`m_is_crashing`/`m_priority`/
/// `m_cpu_affinity`/`m_pending_signals`/`m_signal_mask`/
/// `m_currently_handled_signal`, dan keadaan logis `Thread::Blocker`.
///
/// Modul ini memindahkan **state + transisi + keanggotaan + siklus hidup**
/// tersebut menjadi milik Rust, dalam dua tabel berbatas (`MAX_*` 4096, 64 slot
/// teratas dicadangkan untuk selftest) yang dialamati handle
/// `(index, generation)` yang disimpan di objek C++.
///
/// **BUKAN** pohon pointer-heavy: `Process`/`Thread` tetap objek C++
/// (`ListedRefCounted`/`RefPtr`/`IntrusiveList`), `all_instances`,
/// `for_each_*`, `from_pid_*`, `from_tid_*` tetap C++; konteks CPU/arsitektur
/// (`ThreadRegisters`/`RegisterState`/`FPUState`/`TrapFrame`) tetap C++; dan
/// mekanisme blokir (`BlockerSet`/`FutexBlocker`/`FileBlocker`/…) tetap C++.
///
/// **BUKAN** duplikat `buku_besar_proses` (P1.46): modul itu tetap memiliki
/// registry `(pid, ppid, kernel?)` + agregat; modul ini memiliki KEADAAN
/// objek yang dialamati handle. Keduanya berbagi satu peta: registry = pid→ppid,
/// graf = handle→keadaan.
///
/// `allow(unsafe_code)` dengan alasan yang sama seperti modul lain; tidak ada
/// blok `unsafe` di badan modul (input skalar, atomik).
#[allow(unsafe_code)]
pub mod graf_proses_benang;

/// P1.54 — **otoritas logis PENGHALANG BENANG** (`Thread::Blocker`).
///
/// Memindahkan empat fakta logis blokir dari `KlatKernel/Efek/Eksekusi/Thread.h`
/// (`m_is_blocking`, `m_was_interrupted_by_signal`, `m_was_interrupted_by_death`,
/// `m_did_timeout`) dan keputusan `block_result()` ke Rust, dalam tabel
/// berbatas yang dialamati handle `(index, generation)`. Objek `Blocker` C++
/// tetap pemilik identitas + mekanisme (vtable, `BlockerSet`, `setup_blocker`,
/// `was_unblocked`, `override_timeout`, `can_be_interrupted`).
///
/// **BUKAN** duplikat [`graf_proses_benang`]: modul itu memiliki bendera
/// blokir **tingkat benang** (`TF_BLOCKING`/`TF_INTR_DEATH`/`TF_DID_TIMEOUT`
/// pada tabel benang, dipakai `Thread::block_result()`), sedangkan modul ini
/// memiliki keadaan **objek penghalang** (`Thread::Blocker`) yang hidup hanya
/// selama satu operasi blokir. Keduanya adalah otoritas berbeda atas objek
/// berbeda.
///
/// `allow(unsafe_code)` dengan alasan yang sama seperti modul lain; tidak ada
/// blok `unsafe` di badan modul (input skalar, atomik).
#[allow(unsafe_code)]
pub mod penghalang_benang;

/// P1.56 — **otoritas ANTREAN JALAN PENJADWAL** (`ThreadReadyQueues`).
///
/// Memindahkan keanggotaan, urutan FIFO, ember prioritas, kelayakan afinitas,
/// dan pemilihan benang berikutnya dari `KlatKernel/Efek/Eksekusi/Scheduler.cpp` ke Rust,
/// di atas tabel berbatas `(index, generation)`. C++ tetap memiliki objek
/// `Thread`, CPU, dan context switch; keputusan ember sudah Rust sejak P1.01.
///
/// Kelayakan (`TF_ACTIVE`, `TW_AFFINITY`) **dibaca** dari
/// [`graf_proses_benang`] — bukan disalin (mandat §21: tanpa otoritas ganda).
///
/// `allow(unsafe_code)` dengan alasan yang sama seperti modul lain; tidak ada
/// blok `unsafe` di badan modul (input skalar, atomik).
#[allow(unsafe_code)]
pub mod jadwal_antrean;

/// P1.58 — **OTORITAS KEADAAN LOGIS WILAYAH (`Region`)**.
///
/// Memindahkan keadaan logis sebuah wilayah dari `KlatKernel/Efek/Memori/Region.{h,cpp}`
/// ke Rust: bit akses (`Read`/`Write`/`Execute`), bendera
/// (`shared`/`stack`/`mmap`/`syscall_region`/`mmapped_from_*`), tipe memori,
/// dan riwayat SetOnce (`immutable`/`initially_loaded_executable_segment`/
/// `has_been_*`). C++ menyimpan handle `(index, generation)`.
///
/// **BUKAN** mekanisme halaman: `m_page_directory`, `m_range`,
/// `m_offset_in_vmobject`, `m_vmobject`, `m_name`, pohon intrusive, dan
/// page-fault MEKANIS (`handle_cow_fault`/`handle_inode_fault`/
/// `handle_zero_fault`) tetap C++.
///
/// `allow(unsafe_code)` dengan alasan yang sama seperti modul lain; tidak ada
/// blok `unsafe` di badan modul (input skalar, atomik).
#[allow(unsafe_code)]
pub mod keadaan_wilayah;

/// P1.59 — **OTORITAS KEPUTUSAN PENEMPATAN RUANG WILAYAH**.
///
/// Memindahkan kebijakan penempatan alamat virtual dari
/// `KlatKernel/Efek/Memori/RegionTree.cpp` (`allocate_from_window`, `allocate_range_specific`,
/// `allocate_range_randomized` per-cobaan) dan `KlatKernel/Efek/Memori/VirtualRange.cpp`
/// (`carve`) ke Rust. Murni keputusan: tanpa pointer, tanpa keadaan, sehingga
/// tidak ada sumber kebenaran kedua di samping pohon merah-hitam C++.
///
/// `allow(unsafe_code)` dengan alasan yang sama seperti modul lain; tidak ada
/// blok `unsafe` di badan modul (semua argumen/hasil skalar/POD).
#[allow(unsafe_code)]
pub mod tempat_wilayah;

/// P1.60 — **OTORITAS ADMISI & KLASIFIKASI PAGE FAULT**.
///
/// Memindahkan keputusan jalur page-fault dari `KlatKernel/Efek/Memori/Region.cpp`
/// (gerbang batas+izin `handle_fault`, klasifikasi jenis zero/CoW/inode) dan
/// `Jowo/Arsitektur/PageFault.cpp` (klasifikasi sinyal + properti coredump
/// `fault_access`) ke Rust. Murni keputusan: tanpa pointer, tanpa keadaan.
/// Mekanisme (`handle_zero_fault`, `handle_cow_fault`, `handle_inode_*`,
/// frame sinyal, tabel halaman) tetap C++.
///
/// `allow(unsafe_code)` dengan alasan yang sama seperti modul lain; tidak ada
/// blok `unsafe` di badan modul (semua argumen/hasil skalar/POD).
#[allow(unsafe_code)]
pub mod admisipf;

/// P1.61 — **OTORITAS PENJAGA PAGE FAULT KERNEL**.
///
/// Memindahkan gerbang pertama `MemoryManager::handle_page_fault` (apakah
/// alamat fault di dalam seksi kernel terproteksi `READONLY_AFTER_INIT` /
/// `UNMAP_AFTER_INIT` / `KSYMS`, atau fault di dalam handler IRQ) dari
/// `KlatKernel/Efek/Memori/MemoryManager.cpp` ke Rust. Mekanisme (pembacaan alamat simbol
/// linker, simbolisasi, dump region, pencarian Region) tetap C++.
///
/// `allow(unsafe_code)` dengan alasan yang sama seperti modul lain; tidak ada
/// blok `unsafe` di badan modul (semua argumen/hasil skalar).
#[allow(unsafe_code)]
pub mod penjaga_pf;

/// P1.62 — **OTORITAS AKUNTANSI MEMORI (BOOKKEEPING)**.
///
/// Memindahkan kebijakan "halaman/region ini dihitung sebagai apa" dari
/// `KlatKernel/Efek/Memori/Region.cpp` (`amount_resident`, `amount_shared`,
/// `amount_dirty`) dan `KlatKernel/Efek/Memori/AddressSpace.cpp` (`amount_dirty_private`,
/// `amount_clean_inode`, `amount_purgeable_*`) ke Rust. Mekanisme (iterasi
/// halaman, penjumlahan byte, HashTable VMObject) tetap C++.
///
/// `allow(unsafe_code)` dengan alasan yang sama seperti modul lain; tidak ada
/// blok `unsafe` di badan modul (semua argumen/hasil skalar).
#[allow(unsafe_code)]
pub mod akuntansi_memori;
/// P1.63 — **OTORITAS MODE BERKAS PEMBUATAN**.
///
/// Memindahkan kebijakan `mode & ~umask()` (saat `open(O_CREAT)`/`mkdir`/
/// `mknod`) dan izin `mknod` non-superuser (REG/FIFO/SOCK saja) dari
/// `Jowo/Panggilan/open.cpp:81`, `Jowo/Panggilan/mkdir.cpp:23`,
/// `Jowo/Panggilan/mknod.cpp:25-28` ke Rust. C++ tetap pemilik VFS call
/// (`VirtualFileSystem::open/mkdir/mknod`) dan sanitasi `req & 0o777`.
///
/// `allow(unsafe_code)` dengan alasan yang sama seperti modul lain; tidak ada
/// blok `unsafe` di badan modul (semua argumen/hasil skalar).
#[allow(unsafe_code)]
pub mod mod_berkas;
/// P1.64 — **OTORITAS CHMOD PRODUKSI**.
///
/// Memindahkan keputusan izin `chmod()` (bukan pemilik + bukan superuser ⇒
/// `EPERM`; readonly ⇒ `EROFS`) dan masking bit izin `chmod()`
/// (`(current & ~0o7777) | (new & 0o7777)`) dari
/// `KlatKernel/Efek/Berkas/VirtualFileSystem.cpp:601-613` ke Rust. C++ hanya
/// meneruskan `uid_caller`/`uid_inode`/`is_superuser`/`is_readonly` ke
/// `klatos_izin_chmod`, dan `current_mode`/`new_mode` ke
/// `klatos_mode_efektif_chmod`. `custody.is_readonly()` (C++) memberi
/// `is_readonly`, dan `inode.chmod()` C++ yang menulis hasil kembali ke
/// inode.
///
/// `allow(unsafe_code)` dengan alasan yang sama dengan modul lain; tidak ada
/// blok `unsafe` di badan modul (semua argumen/hasil skalar).
#[allow(unsafe_code)]
pub mod mod_chmod;

/// Memindahkan keputusan izin `chown()` (bukan pemilik + bukan superuser ⇒
/// `EPERM`; `a_uid != NO_CHANGE` dan `euid != a_uid` (bukan superuser) ⇒
/// `EPERM`; `a_gid != NO_CHANGE` dan bukan anggota grup (bukan superuser)
/// ⇒ `EPERM`; `custody.is_readonly()` ⇒ `EROFS`) dan stripping bit
/// `04000 | 02000` ketika SUID/SGID terpasang (`current_mode & !0o6000`)
/// dari `KlatKernel/Efek/Berkas/VirtualFileSystem.cpp:720-753` ke Rust. C++ hanya
/// meneruskan `euid`/`uid_inode`/`a_uid`/`a_gid`/`in_group_a_gid`/
/// `is_superuser`/`is_readonly` ke `klatos_izin_chown`, dan
/// `current_mode`/`has_setid` ke `klatos_mode_strip_setids`. `inode.chmod()`
/// C++ yang menulis hasil mode kembali ke inode.
///
/// `allow(unsafe_code)` dengan alasan yang sama dengan modul lain; tidak ada
/// blok `unsafe` di badan modul (semua argumen/hasil skalar).
#[allow(unsafe_code)]
pub mod mod_chown;

/// Otoritas mutasi entri direktori (P1.66) — `KlatKernel/src/mod_hapus.rs`.
///
/// Memindahkan keputusan izin `unlink()`/`rmdir()` (tulis induk + sticky),
/// `rename()` (tulis kedua induk + sticky induk lama + sticky induk tujuan),
/// dan `link()` (+ `hard_link_allowed`) dari
/// `KlatKernel/Efek/Berkas/VirtualFileSystem.cpp` ke Rust.
///
/// `allow(unsafe_code)` dengan alasan yang sama dengan modul lain; tidak ada
/// blok `unsafe` di badan modul (semua argumen/hasil skalar).
#[allow(unsafe_code)]
pub mod mod_hapus;

/// Memindahkan keputusan admisi ruang alamat pengguna dari C++ ke Rust:
/// gerbang masuk syscall (`validate_syscall_preconditions`), kesahihan stack
/// pengguna (`validate_user_stack`), predikat rentang terlindungi
/// (`is_allowed_to_read_physical_memory_for_userspace`), izin unmap mmap +
/// ukuran/rentang (`AddressSpace::unmap_mmap_range`), validasi
/// offset/ukuran VMObject (`allocate_region_with_vmobject`), dan predikat
/// luber perluasan rentang (`expand_to_page_boundaries`).
///
/// `allow(unsafe_code)` dengan alasan yang sama dengan modul lain; tidak ada
/// blok `unsafe` di badan modul (semua argumen/hasil skalar).
#[allow(unsafe_code)]
pub mod admisi_ruang;

/// Memindahkan keputusan keadaan COW / purge / volatile `AnonymousVMObject`
/// dari C++ ke Rust: predikat `should_cow` (`AnonymousVMObject.cpp:300`),
/// predikat bersama `reset_cow_map`/hitungan `try_clone`, gerbang `purge`
/// (`AnonymousVMObject.cpp:191`), transisi `set_volatile`
/// (`AnonymousVMObject.cpp:218`), dan pohon keputusan `handle_cow_fault`
/// (`AnonymousVMObject.cpp:328`).
///
/// `allow(unsafe_code)` dengan alasan yang sama dengan modul lain; tidak ada
/// blok `unsafe` di badan modul (semua argumen/hasil skalar).
#[allow(unsafe_code)]
pub mod vmo_anon;

/// Memindahkan keadaan halaman `VMObject`/`InodeVMObject` dan keputusan
/// pemetaan halaman `Region` dari C++ ke Rust: jumlah halaman dari ukuran
/// (`VMObject::try_create_physical_pages`), akuntansi halaman bersih/kotor
/// (`InodeVMObject::amount_clean`/`amount_dirty`), predikat pelepasan halaman
/// bersih (`InodeVMObject::try_release_clean_pages`), writable PTE
/// (`Region::map_individual_page_impl`) dan `Region::should_dirty_on_write`.
///
/// `allow(unsafe_code)` dengan alasan yang sama dengan modul lain; tidak ada
/// blok `unsafe` di badan modul (semua argumen/hasil skalar).
#[allow(unsafe_code)]
pub mod vmo_inode;

/// P1.70 / WS1 — otoritas pencarian & cakupan wilayah (`RegionTree`/
/// `AddressSpace` bookkeeping). Lihat [`wilayah_cari`].
#[allow(unsafe_code)]
pub mod wilayah_cari;

/// P1.71 / WS1 — otoritas akuntansi halaman fisik (commit/uncommit/used) dan
/// invariant `physical_pages == used + committed + uncommitted`
/// (`MemoryManager`). Lihat [`akuntansi_fisik`].
#[allow(unsafe_code)]
pub mod akuntansi_fisik;

/// P1.74 / WS4 — otoritas siklus hidup & admisi pemasangan (mount): mesin
/// keadaan pencocokan hierarki path absolut, admisi titik pemasangan
/// (`EBUSY`), kebijakan mount immutable (`EPERM`), dan verdict pelepasan
/// (`ENODEV`). Lihat [`siklus_pasang`].
#[allow(unsafe_code)]
pub mod siklus_pasang;

/// P1.75 / WS3 — otoritas kebijakan dispatch panggilan sistem: admisi
/// dispatch (mati-sistem / di luar `__Count` / handler nol), rute handler
/// (exit/tidak-kembali, regstate fork/sigreturn, biasa), dan keputusan
/// menjatuhkan proses karena pelanggaran janji (`EPROMISEVIOLATION`).
/// **Nomor panggilan sistem & `needs_lock` TIDAK disentuh** (ABI v1 FROZEN).
/// Lihat [`kebijakan_panggilan`].
#[allow(unsafe_code)]
pub mod kebijakan_panggilan;

/// **GENERASI ABI NATIF KLATOS (generasi 3)** — otoritas rute generasi
/// (memisahkan ruang nomor natif dari ABI v1 kompatibilitas 0..163) dan
/// jembatan otoritas KSM (pengganti natif `pledge`/`unveil`: pertanyaan
/// capability, bukan janji). Lihat [`panggilan_native`].
#[allow(unsafe_code)]
pub mod panggilan_native;

/// P1.76 / WS6 — otoritas siklus hidup & admisi SOKET: kelas domain
/// (`EAFNOSUPPORT`), admisi/aksi `shutdown` (`ENOTCONN`, idempoten), admisi
/// antrean koneksi (`ECONNREFUSED`), kelayakan `accept`, dan gerbang
/// `read`/`write` (`EPIPE`). Mekanisme `Socket`/`Vector`/`Mutex` tetap C++.
/// Lihat [`siklus_soket`].
#[allow(unsafe_code)]
pub mod siklus_soket;

/// P1.77 / WS7 — otoritas ADMISI & KEBIJAKAN FORMAT `execve`: kedalaman
/// rekursi `#!` (`ELOOP`), admisi metadata (`EACCES`/`ENOEXEC`), admisi header
/// ELF (`ENOEXEC`), admisi argc/env (`E2BIG`/`EINVAL`), deteksi `#!`, verdict
/// baris `#!` (`ENOBUFS`/`ENOEXEC`), dan validasi ukuran tumpukan. Mekanisme
/// pemetaan ELF / penyusunan `auxv` / penyalinan buffer tetap C++.
/// Lihat [`admisi_exec`].
#[allow(unsafe_code)]
pub mod admisi_exec;

/// P1.78 / WS6 — otoritas ADMISI PANGGILAN SISTEM SOKET: hak soket mentah
/// (`EACCES`), normalisasi `backlog`, admisi `listen`/`shutdown` (`EINVAL`),
/// penerapan flag `accept4`, batas `iovlen` `sendmsg` (`EINVAL`) / bentuk
/// `recvmsg` (`ENOTSUP`), ukuran nama (`EINVAL`), dan domain/protokol
/// `socketpair` (`EINVAL`). Mekanisme fd/`OpenFileDescription`/penyalinan
/// buffer tetap C++. Lihat [`panggilan_soket`].
#[allow(unsafe_code)]
pub mod panggilan_soket;

/// P1.79 / WS5 — otoritas ADMISI PERANGKAT & TTY: admisi buka perangkat
/// terjail (`EPERM`), admisi `TIOCSPGRP` (`EINVAL`/`EPERM`), penerapan
/// `TCFLSH` (`EINVAL`), dan perubahan ukuran `TIOCSWINSZ` (`SIGWINCH`).
/// Mekanisme `ProcessGroup`/`flush_input`/`generate_signal` tetap C++.
/// Lihat [`admisi_perangkat`].
#[allow(unsafe_code)]
pub mod admisi_perangkat;

/// P1.80 / WS4 — otoritas ADMISI VFS: enam verdict pintu `VirtualFileSystem`
/// (`open` awal `EINVAL`, rantai `open` pasca-resolve, `create`, `mknod`,
/// `mkdir`, `utime`/`utimensat`). Mekanisme `resolve_path`/veil/`create_child`/
/// `OpenFileDescription` tetap C++. Izin per-inode sudah Rust (P1.29–P1.33)
/// dan masuk di sini sebagai FAKTA. Lihat [`admisi_vfs`].
#[allow(unsafe_code)]
pub mod admisi_vfs;

/// P1.81 / WS4 — otoritas ADMISI VFS, batch 2: `access()` (`R_OK`/`W_OK`/
/// `X_OK`), `open_directory()`, `symlink()`, dan `safe_to_follow_symlink()`
/// (keputusan symlink pada jalur resolusi path yang TERPANAS). Mekanisme
/// `resolve_path`/veil/`lookup`/`create_child` tetap C++. Lihat
/// [`admisi_vfs2`].
#[allow(unsafe_code)]
pub mod admisi_vfs2;

/// P1.82 / WS4 — otoritas ADMISI STRUKTURAL `rmdir`: `EINVAL` (komponen
/// terakhir `"."`), `ENOTDIR` (bukan direktori), `EBUSY` (tanpa induk =
/// rmdir root), dan `ENOTEMPTY` (jumlah anak ≠ 2). Verdict `ENOTEMPTY`
/// dipecah ke fungsi kedua karena ia duduk DI ANTARA dua pemanggilan Rust
/// P1.66 (`izin_hapus`, `hapus_erofs`). Mekanisme `resolve_path`/
/// `traverse_as_directory`/`remove_child` tetap C++. Lihat [`admisi_rmdir`].
#[allow(unsafe_code)]
pub mod admisi_rmdir;

/// P1.83 / WS4 — otoritas ADMISI STRUKTURAL `rename`: `EPERM` (induk hilang),
/// perlu-telusur + `ENOTEMPTY` (direktori tujuan tak kosong), `EXDEV` (lintas
/// fs), `EDIRINTOSELF` (tujuan di dalam sumber), `EINVAL` (nama kosong/`.`/`..`),
/// no-op-sukses, dan `EISDIR` (direktori ke non-direktori). Mekanisme
/// `resolve_path`/`traverse_as_directory`/`fs().rename()` tetap C++. Lihat
/// [`admisi_rename`].
#[allow(unsafe_code)]
pub mod admisi_rename;

/// P1.84 / WS4 — otoritas ADMISI resolusi path: recursion-limit ELOOP,
/// per-component ENOTDIR/EACCES, dan kebijakan terminal symlink O_NOFOLLOW.
/// Iterasi lexer, lookup, custody, mount, target symlink, recursion, dan veil
/// tetap mekanisme C++. Lihat [`admisi_resolusi`].
#[allow(unsafe_code)]
pub mod admisi_resolusi;

/// P1.85 / WS4 — otoritas ADMISI VFS batch 3: verdict struktural
/// `link()` (EEXIST/ENOENT/EXDEV) dan `unlink()` (EISDIR), plus seluruh
/// validasi mount/device (ENOTSUP/ENOTBLK/ENODEV). Penelusuran path, tabel
/// `s_initializers`, `fsid()`, veil, dan efek inode tetap mekanisme C++.
/// Lihat [`admisi_vfs3`].
#[allow(unsafe_code)]
pub mod admisi_vfs3;

/// P1.86 / WS4 — otoritas GERBANG IZIN VEIL per-akses: verdict
/// `validate_path_against_process_veil` (ENOENT/EACCES) beserta alasan untuk
/// pemilihan pesan. Penelusuran trie veil dan efek `dbgln` tetap mekanisme C++.
/// Lihat [`ksm_jalur`].
#[allow(unsafe_code)]
pub mod ksm_jalur;

/// P1.86 / WS4 — otoritas ADMISI VFS batch 4: `copy_mount` EINVAL,
/// `open()` perangkat ENODEV, jalur kosong EINVAL, pencarian mount ENODEV,
/// plus gerbang tipe/keadaan `OpenFileDescription` (EOVERFLOW/ENOTDIR/EIO/
/// EBADF/ENOENT). Lihat [`admisi_vfs4`].
#[allow(unsafe_code)]
pub mod admisi_vfs4;

/// P1.87 / WS4 — otoritas BERKAS-SISTEM batch 1: verdict Ext2FS (nama,
/// tambah anak, cari, read-only, waktu, link, resize, induk, superblok,
/// block group, blok inode, bitmap, unmount, cache) dan ioctl `InodeFile`.
/// I/O blok, `traverse_as_directory`, dan efek inode tetap mekanisme C++.
/// Lihat [`admisi_fs`].
#[allow(unsafe_code)]
pub mod admisi_fs;

/// P1.88 / WS4 — otoritas BERKAS-SISTEM batch 2: verdict mount-option
/// (`MountFile`), penonton inode (`InodeWatcher`), FIFO, bawaan `File`
/// (ioctl/mmap), dan `BlockBasedFileSystem`. I/O, `KString`, `DoubleBuffer`,
/// dan `validate_mount_*_flag` tetap mekanisme C++.
/// Lihat [`admisi_fs2`].
#[allow(unsafe_code)]
pub mod admisi_fs2;

/// P1.89 / WS4 — otoritas BERKAS-SISTEM batch 3: verdict FATFS (signature
/// boot sector/BPB DOS7/FSInfo, klaster habis, klaster tak valid, traverse/
/// lookup/remove gagal, mode create_child buruk, operasi tak didukung) dan
/// ISO9660FS (read-only, struktur tak valid, lookup gagal). I/O blok,
/// `SFNUtils`, dan transformasi data tetap mekanisme C++.
/// Lihat [`admisi_fs3`].
#[allow(unsafe_code)]
pub mod admisi_fs3;

/// P1.90 / WS4 — otoritas BERKAS-SISTEM batch 4: verdict Plan9FS (ENOTIMPL/
/// EINTR/EIO), FUSE (EIO/EINVAL), DevPtsFS + DevLoopFS (EROFS/ENOENT/ENOTDIR),
/// dan `UnsharedResourceFile` (ENOTSUP/ENOTDIR/EACCES/EINVAL). Relai kode
/// errno dari server jauh tetap mekanisme C++.
/// Lihat [`admisi_fs4`].
#[allow(unsafe_code)]
pub mod admisi_fs4;

/// P1.91 / WS4 — otoritas BERKAS-SISTEM batch 5: verdict `ProcFS` (`/proc`) —
/// EROFS rename, EINVAL traverse proses hilang, ESRCH/ENOENT lookup, EIO tanpa
/// cache, ESRCH/EPERM refresh, ENOENT lookup gagal, ESRCH stack, ENOBUFS perf,
/// ENOEXEC binary. Serialisasi JSON dan alokator tetap mekanisme C++.
/// Lihat [`admisi_proc`].
#[allow(unsafe_code)]
pub mod admisi_proc;

/// P1.92 / WS4 — otoritas BERKAS-SISTEM batch 6: verdict `SysFS` (`/sys`) —
/// EROFS tulis, EPERM chmod/chown, ENOENT komponen, ELOOP hop, EPERM jail,
/// EIO data hilang, EINVAL offset/boolean/coredump/power, EOVERFLOW,
/// ENOTSUP request-panic. Pemetaan mode dan serialisasi JSON tetap C++.
/// Lihat [`admisi_sysfs`].
#[allow(unsafe_code)]
pub mod admisi_sysfs;

/// P1.93 / WS4 — otoritas PEMASANGAN + VALIDASI PERANGKAT: EINVAL bendera mount
/// tak didukung, ENODEV perangkat loop hilang, EINVAL offset, EOVERFLOW ukuran
/// `devctl`, EINVAL versi protokol, EINVAL operasi/ioctl tak dikenal. I/O driver
/// dan protokol wire NullServer tetap C++.
/// Lihat [`admisi_pasang`].
#[allow(unsafe_code)]
pub mod admisi_pasang;

/// P1.94 / WS6 — otoritas JARINGAN batch 1: bidang kendali soket IPv4 —
/// ENOPROTOOPT opsi/level tak dikenal, EINVAL panjang nilai sockopt (persis &
/// minimum), ENODEV/ENOTSUP/EFAULT `SO_*`, EINVAL nilai `IP_TTL`/`IP_TOS`/
/// `IP_MULTICAST_LOOP`, ENOTSUP antarmuka multicast, EPERM/EAFNOSUPPORT/EINVAL/
/// ENODEV `ioctl` rute/ARP/antarmuka, EINVAL/EISCONN/EACCES/EPIPE admisi alamat
/// `bind`/`connect`/`sendto`. Transport TCP/UDP dan `LocalSocket` menyusul.
/// Lihat [`admisi_net`].
#[allow(unsafe_code)]
pub mod admisi_net;

/// P1.95 / WS6 — otoritas JARINGAN batch 2: transport + soket lokal —
/// ENOPROTOOPT sockopt TCP, EEXIST tuple, EADDRNOTAVAIL adapter,
/// EADDRINUSE port, ETIMEDOUT/ECONNREFUSED/EINPROGRESS `connect` TCP, dan
/// verdict `LocalSocket` (EINVAL/EADDRINUSE/EISCONN/ECONNREFUSED/EOPNOTSUPP/
/// EPIPE/ENOTCONN/EPERM/EBUSY/EAGAIN), plus EEXIST/ESRCH tabel rute dan EINVAL
/// tipe soket IPv4. Panjang `SO_SNDBUF`/`SO_RCVBUF`/`SO_PEERCRED` memakai ulang
/// verdict P1.94.
/// Lihat [`admisi_net2`].
#[allow(unsafe_code)]
pub mod admisi_net2;

/// P1.96 / WS6 — otoritas PROSES batch 1: kendali proses + identitas —
/// tangga penolakan `ptrace` (ESRCH/EPERM/EACCES/EBUSY/EINVAL/EFAULT), indeks
/// register debug, aturan POSIX `setpgid` (EINVAL/ESRCH/EPERM), rentang sinyal
/// `kill`/`killpg`, grup kosong, proses inti, validasi `prctl` (EINVAL), sentinel
/// `(id_t)-1`, batas `NGROUPS_MAX`, izin `setgroups` (EPERM). Otoritas identitas
/// (P1.29) dan task (P0.79/P0.80) tetap Rust; ini lapisan validasi di atasnya.
/// Lihat [`admisi_proses`].
#[allow(unsafe_code)]
pub mod admisi_proses;

/// P1.97 / WS6 — otoritas PENJADWAL & THREAD batch 2: izin lintas-proses
/// `sched_*` (EPERM/ESRCH), overflow stack + validasi stack pengguna + rentang
/// prioritas + detach tak-joinable + join diri + rentang sinyal `kill_thread`
/// (EOVERFLOW/EFAULT/EINVAL/EDEADLK), `ECHILD` dari `WaitBlocker`, id
/// `ScopedProcessList` negatif, `waitid` (EINTR/ECHILD/EINVAL), `posix_spawn`
/// (E2BIG/EINVAL/ENOTSUP/EOVERFLOW), `disown` (ESRCH/ECHILD). `izin_jadwal`
/// (P1.32) tetap Rust; ini lapisan validasi di atasnya. Lihat [`admisi_jadwal`].
#[allow(unsafe_code)]
pub mod admisi_jadwal;

/// P1.98 — otoritas signal, profiling, dan coredump. Lihat [`admisi_sinyal2`].
#[allow(unsafe_code)]
pub mod admisi_sinyal2;

/// P1.99 — futex + hostname + uid/group miscellaneous authority. Lihat [`admisi_misc`].
#[allow(unsafe_code)]
pub mod admisi_misc;

/// P2.01A — mmap argument/file admission. Lihat [`admisi_mmap2`].
#[allow(unsafe_code)]
pub mod admisi_mmap2;

/// P2.01B — memory/VM admission batch 2 (sisa keluarga mmap + gerbang
/// penempatan wilayah). Lihat [`admisi_mem2`].
#[allow(unsafe_code)]
pub mod admisi_mem2;

/// P2.01C — memory/VM admission batch 3 (VMObject / RingBuffer). Lihat
/// [`admisi_mem3`].
#[allow(unsafe_code)]
pub mod admisi_mem3;

/// P2.02 — security / capability admission batch 1 (privilege / veil / jail /
/// socket-type). Lihat [`admisi_aman`].
#[allow(unsafe_code)]
pub mod admisi_aman;

/// P2.03 — IPC channel admission (endpoint queue / pipe / dup2). Lihat
/// [`admisi_kanal`].
#[allow(unsafe_code)]
pub mod admisi_kanal;

/// P2.04 — filesystem admission batch 1 (RAMFS / base `Inode` / Ext2FS
/// residuals). Lihat [`admisi_berkas`].
#[allow(unsafe_code)]
pub mod admisi_berkas;

/// P2.05 — filesystem admission batch 2 (FUSE + residual one-liners). Lihat
/// [`admisi_berkas2`].
#[allow(unsafe_code)]
pub mod admisi_berkas2;

/// P2.06 — `execve` admission batch 2 (ELF image loading). Lihat
/// [`admisi_exec2`].
#[allow(unsafe_code)]
pub mod admisi_exec2;

/// P2.07 — process/identity admission batch 2 (process object + perf buffer).
/// Lihat [`admisi_proses2`].
#[allow(unsafe_code)]
pub mod admisi_proses2;

/// P2.08 — network socket admission residuals. Lihat [`admisi_jaring`].
#[allow(unsafe_code)]
pub mod admisi_jaring;

/// P2.09 — generic syscall admission residuals (read/write/fallocate/clock/
/// chdir/keymap/poll/fcntl). Lihat [`admisi_panggilan`].
#[allow(unsafe_code)]
pub mod admisi_panggilan;

/// P2.10 — generic syscall admission residuals batch 2 (alarm/ftruncate/
/// inode_watcher/readlink/resource/statvfs/sysconf/utimensat). Lihat
/// [`admisi_panggilan2`].
#[allow(unsafe_code)]
pub mod admisi_panggilan2;

/// P1.101 / REHOME-01 — **otoritas AKSI BADAN HANDLER panggilan sistem**:
/// `sysconf` (nama `_SC_*` → sumber nilai), `ioctl` (request → kelas),
/// `dbgputstr` (ukuran → kelas buffer), `unlink` (flags → operasi),
/// `umask` (mask → topeng). Kelima keputusan ini sebelumnya hidup sebagai
/// `switch`/`if` di dalam BADAN handler C++ `Jowo/Panggilan/*.cpp`; sejak
/// P1.101 C++ hanya menjalankan MEKANISME (VFS/OpenFileDescription/penyalinan
/// user). Lihat [`panggilan_aksi`].
#[allow(unsafe_code)]
pub mod panggilan_aksi;

/// P1.102 / REHOME-02 — **otoritas AKSI & KLASIFIKASI `Jowo/Keamanan`**:
/// keanggotaan grup `Credentials::in_group`, MEKANISME penguraian spesifikasi
/// janji (28 janji `ENUMERATE_KSM_PERMISSIONS`), klasifikasi KASAN (kebijakan
/// deadly + tabel `AccessType`/`ShadowType`), dan klasifikasi UBSAN (lokasi /
/// deadly / pointer-overflow / type-mismatch / alignment) + tabel `kinds[]`.
/// C++ yang tersisa: `FixedArray`, thunk ABI compiler `__asan_*`/`__ubsan_*`,
/// format pesan, `Processor::halt`. Gerbang/ukuran + memori-bayangan KASAN ada
/// di [`kasan_bayangan`]. Lihat [`keamanan_aksi`].
#[allow(unsafe_code)]
pub mod keamanan_aksi;

/// P1.103 / REHOME-02b — **MEKANISME memori-bayangan KASAN** milik Rust:
/// keadaan (`shadow_base`/`shadow_offset`/`initialized`/`mapping_base`),
/// `va_to_shadow`, `fill_shadow`, `mark_region`, dan `shadow_va_check`
/// (tiga gerbang + pemilihan + algoritma `{1,2,4,8,N}b`). C++ yang tersisa
/// hanya thunk ABI compiler `__asan_*`, penerus batas `AddressSanitizer::`
/// untuk `Jowo/Memori`, dan `print_violation`. Lihat [`kasan_bayangan`].
#[allow(unsafe_code)]
pub mod kasan_bayangan;

/// P1.104 / REHOME-02b — **MEKANISME PRNG `Fortuna`** milik Rust: keadaan
/// (`key`/`counter`/`reseed_number`/`p0_len`/32 kolam), `reseed`, dan
/// `get_random_bytes` (AES-256-CTR), termasuk primitif [`fortuna::sha256`] dan
/// [`fortuna::aes256`]. C++ yang tersisa hanya `Spinlock` + `DeprecatedWaitQueue`
/// (`Jowo/Sinkronisasi`), sumber entropi boot, dan `get_good_random_bytes`.
#[allow(unsafe_code)]
pub mod fortuna;

/// REHOME-03 — **KEPEMILIKAN MEKANISME trie `unveil`** milik Rust:
/// penyimpanan arena/slab global (simpul + teks), struktur trie/path, inti
/// `update_unveil_data` (traverse + elevasi + propagasi + insert), verdict
/// `path_contains` (longest component-prefix), iterasi tree-order untuk
/// `/proc/<pid>/unveil`, dan `deep_copy`/`clear`. C++ menjadi pemanggil:
/// `Process::UnveilData` hanya menyimpan `state` + handle `root` (`u32`);
/// `Paket::Trie`/`UnveilNode`/`UnveilMetadata` di-retire.
///
/// `allow(unsafe_code)` di sini **nyata**: modul ini memiliki arena global
/// (`UnsafeCell<Arena>` + spinlock) dan pintu C ABI yang menafsirkan
/// `*const u8`/`*mut u8` dari C++. Setiap blok `unsafe` diberi komentar
/// SAFETY. Lihat [`ksm_pohon_jalur`].
#[allow(unsafe_code)]
pub mod ksm_pohon_jalur;

/// FASE-2 KSM — persistent capability/object authority. Rust owns fixed
/// object/capability state, anti-ABA generations, compact-vector swap removal,
/// rights checks, revoke, transfer, transit handoff, and task death. C++
/// endpoint/device/process effects remain separate subsystem boundaries until
/// their consumer migrations complete. The module uses one internal transaction
/// spinlock and scalar/by-value C ABI; it does not call the former C++ table.
#[allow(unsafe_code)]
pub mod ksm_kapabilitas;

/// FASE-6 — pemilik Rust **peta kunci** (ABI v2 `Keymap::GET/SET`). Menggantikan
/// keadaan C++ global `InputManagement::keymap_data()` dengan keadaan Rust
/// (5 peta `u32[0x80]` + nama ≤ 50), keputusan superuser/panjang-nama, dan
/// serialisasi. Hak `Keymap` (baca/tulis) tetap dari tabel dispatch beku. Lihat
/// [`input_keymap`].
#[allow(unsafe_code)]
pub mod input_keymap;

/// FASE-8 — **KEPEMILIKAN KEADAAN `AnonymousVMObject`** (Rust): `m_purgeable`,
/// `m_volatile`, dan `m_was_purged` kini hidup di tabel berbatas yang dialamati
/// handle `(index, generation)` yang disimpan objek C++. Keputusan COW/purge/
/// volatile tetap di [`vmo_anon`]. Lihat [`keadaan_vmoan`].
#[allow(unsafe_code)]
pub mod keadaan_vmoan;

/// FASE-8 — **KEPEMILIKAN IDENTITAS `Credentials`** (Rust): `uid`/`gid`/
/// `euid`/`egid`/`suid`/`sgid`/`sid`/`pgid` + `extra_gids` kini hidup di tabel
/// berbatas yang dialamati handle. Keputusan keanggotaan grup tetap di
/// [`keamanan_aksi::dalam_grup`]. Lihat [`identitas_aksi`].
#[allow(unsafe_code)]
pub mod identitas_aksi;

/// FASE-12 — Rust sole owner untuk bitmap COW AnonymousVMObject.
#[allow(unsafe_code)]
pub mod cow_map;

/// FASE-13R — **KONTEKS EFEK KAPABILITAS MEMORI** (ABI v2). Pemilik byte Rust
/// yang sesungguhnya: arena berbatas tanpa alokator, dialamati handle
/// `(index, generation)`, dengan hak per-slot dan salinan NYATA masuk/keluar.
/// Menutup celah `fileref_baca_tulis` yang hanya mengembalikan `arg2`.
/// Lihat [`memori_efek`].
#[allow(unsafe_code)]
pub mod memori_efek;

/// FASE-13R — **PENYIMPAN BYTE FileRef** (ABI v2 `FileRef::READ/WRITE`).
/// Pemilik byte berkas Rust yang sesungguhnya; potongan vertikal NYATA
/// berkas → memori lewat [`memori_efek`]. Menutup `Ok(request.arg2)` pada
/// `mekanisme_abi2::fileref_baca_tulis`. Lihat [`fileref_efek`].
#[allow(unsafe_code)]
pub mod fileref_efek;

/// FASE-14R (Deliverable C1) — **MESIN KEADAAN CINCIN PENYANGGA**. Memindahkan
/// keadaan `m_start_of_used`/`m_num_used_bytes`/`m_capacity_in_bytes` dan
/// seluruh aritmetika cincin `KlatKernel/Efek/Memori/RingBuffer.cpp` ke Rust. C++ tinggal
/// memiliki `Memory::Region` + `Spinlock`; keputusan admission kepenuhan tetap
/// milik `admisi_mem3`. Lihat [`cincin_penyangga`].
#[allow(unsafe_code)]
pub mod cincin_penyangga;

/// FASE-15B — **IKATAN EFEK: capability KSM → pemilik byte Rust**. Tabel
/// berbatas yang mengikat identitas objek KSM `(kind, index, generation)` ke
/// penyimpan byte Rust (`memori_efek`/`fileref_efek`). Menutup F-P15A-03:
/// jalur ABI v2 `FileRef::READ/WRITE` + `Memory::READ/WRITE` tidak lagi
/// membawa handle penyimpan mentah di `arg0`/`arg3`; byte hanya dapat disalin
/// dari capability yang sudah diloloskan `dispatch()`. Lihat [`ikatan_efek`].
#[allow(unsafe_code)]
pub mod ikatan_efek;

/// FASE-20B — pemilik keadaan `ThreadTracer` (pid/sinyal/trace_syscalls);
/// cuplikan register tetap arch boundary di C++. Lihat [`penjejak_utas`].
#[allow(unsafe_code)]
pub mod penjejak_utas;

/// FASE-19D — pemilik keadaan logis `AddressSpace` (bendera kebijakan
/// `enforces_syscall_regions`); page directory + pohon region tetap C++
/// boundary. Lihat [`keadaan_ruang`].
#[allow(unsafe_code)]
pub mod keadaan_ruang;

/// FASE-20A — pemilik bendera kebijakan `Process` (`jailed_until_exec`,
/// `jailed_until_exit`, `reject_transition_to_executable_from_writable_prot`).
/// Lihat [`keadaan_proses`].
#[allow(unsafe_code)]
pub mod keadaan_proses;

/// F-P8 BATCH A — **MEKANISME JALUR LEKSIKAL**. Memindahkan seluruh algoritma
/// `KlatKernel/Efek/Inti/KLexicalPath.cpp` (`is_absolute`, `is_canonical`, `basename`,
/// `dirname`, `parts`, `try_join`) ke Rust. C++ tinggal adapter ABI tipis yang
/// menyerahkan `(ptr,len)` dan menyusun `StringView` dari offset. Lihat
/// [`jalur_leksikal`].
#[allow(unsafe_code)]
pub mod jalur_leksikal;

/// F-P8 BATCH A — **KEBIJAKAN WILAYAH FISIK**. Memindahkan kebijakan tata-letak
/// zona `KlatKernel/Efek/Memori/PhysicalRegion.cpp` (partisi 16 MiB/1 MiB, order buddy,
/// indeks zona untuk alamat) ke Rust. C++ tetap memiliki objek `PhysicalZone`
/// + `PhysicalRAMPage`. Lihat [`wilayah_fisik`].
#[allow(unsafe_code)]
pub mod wilayah_fisik;

/// F-P8 BATCH C — pemilik keadaan + mekanisme FAT 8.3 SFN. C++ `SFN` tinggal
/// adapter RefCounted atas slot Rust; seluruh translasi long-name, validasi,
/// suffix unik, dan serialisasi dimiliki [`sfn_fat`].
#[allow(unsafe_code)]
pub mod sfn_fat;

/// F-P8 BATCH D — pemilik mask + invariant urutan `Thread::LockRank`.
/// C++ `Thread` hanya mengirim ThreadID/rank ke slot Rust. Lihat
/// [`peringkat_kunci`].
#[allow(unsafe_code)]
pub mod peringkat_kunci;

/// F-P8 BATCH F — **OTORITAS SIKLUS HIDUP `AnonymousFile` + ADMISI BERKAS
/// MAYA**. Rust memiliki identitas objek (arena berbatas dialamati handle
/// `(index, generation)`, generasi anti-ABA), siklus hidup `buat`/`lepas`, dan
/// gerbang admisi ukuran yang sebelumnya hanya predikat murni
/// `admisi_mem2::anon_ukuran`. C++ `AnonymousFile` hanya menyimpan handle;
/// representasi halaman (`AnonymousVMObject`) dan alokasi fd tetap C++. Nomor
/// panggilan sistem `anon_create = 4` (ABI v1 FROZEN) tidak disentuh. Lihat
/// [`siklus_berkas_maya`].
#[allow(unsafe_code)]
pub mod siklus_berkas_maya;

/// F-P8 BATCH 5 (G2) — **SERIALISASI JALUR ABSOLUT CUSTODY**. Memindahkan
/// MEKANISME generik `Custody::try_serialize_absolute_path`
/// (`KlatKernel/Efek/Berkas/Custody.cpp`) ke Rust: penyusunan jalur absolut dari rantai
/// nama custody (urutan root-dulu, pemisah `/`, kasus khusus root, batas
/// panjang). C++ hanya menelusuri `RefPtr<Custody>` parent (integrasi graf
/// objek) dan menyerahkan daftar nama komponen. TIDAK menduplikasi
/// normalisasi leksikal milik [`jalur_leksikal`]. Lihat [`kustodi_jalur`].
#[allow(unsafe_code)]
pub mod kustodi_jalur;

/// F-P8.3 BATCH B — **OTORITAS SIKLUS HIDUP UJUNG + MEKANISME ANTREAN
/// KlatIPC**. Rust memiliki identitas ujung (arena berbatas, handle
/// `(index, generation)`, generasi anti-ABA), siklus hidup
/// `CREATE→ACTIVE→CLOSED→RELEASED`, topologi peer, serta mekanisme antrean
/// FIFO (`SEND→ENQUEUE→RECEIVE→DEQUEUE`) beserta batas/batasnya. C++
/// `Endpoint` hanya menyimpan handle; efek memori (payload) dan efek
/// penjadwalan tetap C++. Otoritas tetap KSM. Lihat [`siklus_ujung`].
#[allow(unsafe_code)]
pub mod siklus_ujung;

/// F-P8.3 BATCH C — **OTORITAS PENUNGGU/BANGUN GENERIK KlatIPC**. Rust
/// memiliki state machine penunggu (`WAITING→QUEUED→WOKEN→RUNNABLE`),
/// identitas penunggu bervgenerasi, deteksi penunggu ganda, dan penolakan
/// bangun ganda/bangun atas penunggu yang sudah keluar (EXITED). Efek
/// blok/bangun thread tetap C++ (`Jowo::WaitQueue`/`Scheduler`); peringkat
/// kunci tetap milik [`peringkat_kunci`] dan TIDAK diduplikasi. Lihat
/// [`penunggu_ujung`].
#[allow(unsafe_code)]
pub mod penunggu_ujung;

/// F-P8.4 BATCH B — **OTORITAS SIKLUS HIDUP PERMINTAAN PERANGKAT (generik)**.
/// Rust memiliki identitas permintaan perangkat (arena berbatas, handle
/// `(index, generation)`, generasi anti-ABA), siklus hidup
/// `Pending→Started→{Success,Failure,MemoryFault}`, dan kebijakan agregasi
/// sub-permintaan (`Failure` > `MemoryFault` > `Success`). C++
/// `AsyncDeviceRequest` hanya menyimpan handle; graf objek/`LockRefPtr`,
/// callback `start()`, efek IRQ/deferred-call, dan antrean perangkat tetap C++
/// (efek/arsitektur). Otoritas tetap KSM. Lihat [`siklus_permintaan`].
#[allow(unsafe_code)]
pub mod siklus_permintaan;

/// F-P8.5 BATCH B — **OTORITAS KEBIJAKAN TIMER TERURUT (generik)**. Rust
/// memiliki identitas timer (arena berbatas, handle `(index, generation)`),
/// urutan `(deadline, seq)`, `next_due`, pemilihan siap-bakar
/// (`fire`), dan siklus hidup `QUEUED → EXEC → free`/batal. C++ `TimerQueue`
/// hanya adapter tipis: objek `Timer`, `Duration`, callback, dan
/// `Processor::deferred_call_queue` tetap C++ (efek). Otoritas tetap KSM.
/// Lihat [`antrian_timer`].
#[allow(unsafe_code)]
pub mod antrian_timer;

/// F-P8.5-D — otoritas urutan inisialisasi boot (generik): manifes langkah
/// eksplisit, dependensi, urutan deterministik, dan state machine
/// `PENDING → RUN → DONE`/gagal. C++ `init.cpp` hanya menjalankan efek tiap
/// langkah; tidak ada tabel urutan di C++. Otoritas tetap KSM.
/// Lihat [`manifes_boot`].
#[allow(unsafe_code)]
pub mod manifes_boot;

/// KERNEL-RUST KR-FS — **OTORITAS OPSI BERKAS-SISTEM (generik)**. Rust memiliki
/// penyimpanan tabel nilai bertag, pencarian kunci, penyisipan/penggantian,
/// penghapusan, validasi/admisi, dan batas. C++ `FileSystemSpecificOptions`
/// hanya handle RAII; `FileSystemSpecificOption` (tunggal) dihapus;
/// `parse_*_filesystem_specific_option` menjadi seam tipis. Lihat [`opsi_berkas`].
#[allow(unsafe_code)]
pub mod opsi_berkas;

/// KERNEL-RUST KR-MUTEX — **OTORITAS MESIN KEADAAN KUNCI (`Jowo::Mutex`)**.
/// Rust memiliki `mode`/`times_locked`/`holder`/`shared_holders`, keputusan
/// `lock()`/`unlock()`, pemilihan daftar penunggu, dan `force_unlock`/
/// `restore_exclusive_lock` khusus BigLock. C++ menyematkan blok POD buram dan
/// tetap memiliki daftar penunggu intrusif, `Thread::block`, `Scheduler::yield`,
/// penjaga IRQ, dan LockRank. Lihat [`kunci_eksklusif`].
#[allow(unsafe_code)]
pub mod kunci_eksklusif;

/// KERNEL-RUST KR-WAIT — **OTORITAS ANTREAN TUNGGU / PEMBENDUNG (generik)**.
/// Rust memiliki keanggotaan/urutan/pemilihan-bangun antrean generik yang
/// dipakai `Jowo::WaitQueue` dan `Jowo::Thread::BlockerSet` (termasuk
/// `FutexQueue`/`DeprecatedWaitQueue`). C++ menyematkan blok POD buram dan
/// tetap memiliki efek penjadwalan, status thread, predikat virtual, penjaga
/// IRQ, dan Spinlock. Lihat [`antrean_tunggu`].
#[allow(unsafe_code)]
pub mod antrean_tunggu;

/// KR-29 — **OTORITAS KEKEKALAN BENDERA MOUNT (sticky-immutable latch)**.
/// Rust memiliki mesin keadaan latch tulis-sekali `Mount::set_flags`
/// (`m_flags` + `SetOnce m_immutable`). C++ menyematkan blok POD buram dan
/// tetap memiliki Spinlock serta efek objek. Lihat [`kebijakan_pasang`].
#[allow(unsafe_code)]
pub mod kebijakan_pasang;

/// KR-29 — **OTORITAS PENDAFTARAN HANDLER IRQ BERSAMA**.
/// Rust memiliki registri handler `SharedIRQHandler` (keanggotaan/urutan,
/// auto-disable bila kosong, latch aktif idempoten). C++ tetap memiliki efek
/// pengendali IRQ dan handler. Lihat [`pendaftaran_irq`].
#[allow(unsafe_code)]
pub mod pendaftaran_irq;

/// KR-29 — **OTORITAS RENCANA KLON WILAYAH (Region::try_clone)**.
/// Rust memiliki keputusan jenis klon (shared vs CoW) + invarian + propagasi
/// efek. C++ menjalankan efek objek. Lihat [`klon_wilayah`].
#[allow(unsafe_code)]
pub mod klon_wilayah;

/// KR-30 — **OTORITAS KEBIJAKAN SISTEM (uname/realpath/utime/nofollow)**.
/// Rust memiliki penyusunan field `utsname`, kebijakan pemotongan `realpath`,
/// admisi stempel-waktu bawaan `utime`, dan pemetaan `follow_symlinks`.
/// Lihat [`kebijakan_sistem`].
#[allow(unsafe_code)]
pub mod kebijakan_sistem;

/// KR-30 — **OTORITAS RUTE & REGISTRI ADAPTER JARINGAN**.
/// Rust memiliki rute loopback IPv4/IPv6, saringan kelas PCI, sintesis nama
/// antarmuka, dan urutan tabel probe driver. Lihat [`siklus_jaringan`].
#[allow(unsafe_code)]
pub mod siklus_jaringan;

/// KR-30 — **OTORITAS GERBANG PROFILING PANGGILAN SISTEM**.
/// Rust memiliki gerbang admission profiling, penyandian hasil `ErrorOr`, dan
/// pemilihan jalur. Lihat [`siklus_profiling`].
#[allow(unsafe_code)]
pub mod siklus_profiling;

/// KR-30 — **OTORITAS PERINTAH & PENGURASAN DAYA**.
/// Rust memiliki pemetaan perintah daya dan mesin keadaan pengurasan proses.
/// Lihat [`siklus_matikan`].
#[allow(unsafe_code)]
pub mod siklus_matikan;

/// KR-30 — **OTORITAS KEBIJAKAN TUGAS (crash/fs-init/finalizer/kustodi)**.
/// Rust memiliki predikat kelayakan sinyal crash, penjaga init-lazy, konsumsi
/// `has_work` finalizer, dan perutean dasar kustodi. Lihat [`kebijakan_tugas`].
#[allow(unsafe_code)]
pub mod kebijakan_tugas;

/// Menulis ke serial COM1 tanpa baris baru. Lihat [`serial::write_fmt`].
/// (lihat [`penunggu_ujung`])
#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {
        $crate::serial::write_fmt(core::format_args!($($arg)*))
    };
}

/// Menulis satu baris ke serial COM1.
#[macro_export]
macro_rules! kprintln {
    () => {
        $crate::serial::write_byte(b'\n')
    };
    ($($arg:tt)*) => {{
        $crate::serial::write_fmt(core::format_args!($($arg)*));
        $crate::serial::write_byte(b'\n');
    }};
}

/// Menyalin ulang identitas mekanisme untuk pemanggil di luar crate.
pub use ksm::MECHANISM_TAG;
