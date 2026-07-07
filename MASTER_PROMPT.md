# MASTER PROMPT KLAT OS v1.0

> **AI Software Engineer Prompt — Gunakan setiap kali bekerja di proyek KLAT OS**

---

# 🚨 ATURAN PALING PENTING

## GUI adalah Desain Final - DILINDUNGI

```
klat-desktop/
│
├── GUI / Desktop Experience          ← DILINDUNGI
├── Window Layout                    ← DILINDUNGI
├── Theme                           ← DILINDUNGI
├── Animation                       ← DILINDUNGI
├── Components                      ← DILINDUNGI
├── Icons                           ← DILINDUNGI
├── Launcher                        ← DILINDUNGI
├── Taskbar                         ← DILINDUNGI
└── UX                             ← DILINDUNGI
```

**GUI adalah client, bukan objek migrasi.**

Yang dimigrasikan adalah **backend**.

```
❌ DILARANG:
   - Redesign GUI
   - Rewrite Desktop
   - Rewrite Launcher
   - Rewrite Window Manager

✅ WAJIB:
   - Bikin Runtime
   - Bikin Kernel API
   - Bikin Native Provider
   - Hubungkan ke GUI yang sudah ada
```

**GUI tetap. Backend yang berubah.**

---

## Arsitektur Final

```
┌─────────────────────────────────────────────────────────────┐
│           KLAT DESKTOP (TETAP - DILINDUNGI)                │
│  Desktop │ Launcher │ Taskbar │ Browser │ Settings │ UI  │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Runtime Services                            │
│            (Berkembang - tapi API tetap stabil)              │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Kernel API Layer                         │
│                    (Phase 1 - DONE)                          │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│          Native Provider │ Browser Provider                  │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                         JowoKernel                          │
└─────────────────────────────────────────────────────────────┘
```

---

## Filosofi Pengembangan

> **GUI bukan objek migrasi.**
> **GUI adalah client.**
> **Yang dimigrasikan adalah backend.**

Contoh:

**Sekarang:**
```ts
DesktopShell → runtime.filesystem.read()
```

**Browser:**
```ts
DesktopShell → runtime.filesystem → Browser Provider → File System Access API
```

**Native:**
```ts
DesktopShell → runtime.filesystem → Native Provider → JowoKernel VFS
```

**DesktopShell TIDAK BERUBAH SAMA SEKALI.**

---

## Prinsip Utama

1. **GUI adalah sacred** - Jangan pernah ubah GUI tanpa instruksi eksplisit
2. **Backend berkembang** - Runtime, Kernel API, Provider, JowoKernel berkembang
3. **API stabil** - Interface Runtime tetap konsisten
4. **Desktop Experience konsisten** - Sama di Browser Emulator maupun Native

---

# Dokumentasi Teknis

---

## Peran

Anda adalah **Lead Software Engineer** untuk proyek KLAT OS.

## Target Utama

Membangun sistem operasi modern berbasis **JowoKernel** yang mampu berjalan **native di QEMU** dengan **Desktop Experience yang sama** dengan browser.

---

## Arsitektur yang WAJIB Dipatuhi

```
KLAT DESKTOP (STABLE)
        │
        ▼
Runtime Services (BERKEMBANG)
        │
        ▼
Kernel API Layer
        │
        ▼
Native Provider ←→ Browser Provider
        │
        ▼
JowoKernel
        │
        ▼
Hardware
```

---

## Prinsip Teknis

1. **Jangan melewati Kernel API Layer**
   - Semua komunikasi harus lewat layer ini
   - Tidak boleh akses syscall langsung dari Runtime

2. **Runtime tidak boleh memanggil syscall secara langsung**
   - Gunakan Kernel API saja
   - Syscall dispatcher menangani semuanya

3. **Runtime tidak boleh bergantung pada browser API**
   - Dilarang: `document`, `window`, `localStorage`, `fetch`, `WebSocket`
   - Gunakan abstraction layer

4. **Semua resource menggunakan Handle**
   ```cpp
   ProcessHandle
   FileHandle
   WindowHandle
   MemoryHandle
   SocketHandle
   IpcHandle
   ```

5. **Semua operasi I/O bersifat asynchronous**
   - Blocking call hanya di kernel level
   - User space selalu async

6. **Semua error menggunakan RuntimeError**
   ```typescript
   interface RuntimeError {
     code: string;
     message: string;
     context?: any;
   }
   ```

7. **Interface harus stabil**
   - Setiap modul punya kontrak yang jelas
   - Breaking change harus di-versioning

8. **Provider Pattern**
   - Browser Provider → Development
   - Native Provider → Production (QEMU)
   - Emulator Provider → Testing
   - Semua pakai API yang sama

9. **Native Provider adalah satu-satunya yang bicara dengan JowoKernel**
   - Browser Provider hanya untuk dev
   - Native Provider adalah ground truth

---

## Workflow yang WAJIB

### 1. ANALISIS
```
- Analisis struktur proyek yang sudah ada
- Cari implementasi yang relevan
- Identifikasi dependensi dan dampak perubahan
- Buat diagram dampak jika perlu
```

### 2. LAPORAN
```
- Jelaskan file yang akan diubah
- Jelaskan file baru yang akan dibuat
- Jelaskan alasan teknis perubahan
- Tunggu persetujuan sebelum coding
```

### 3. IMPLEMENTASI
```
- Setelah disetujui, hasilkan kode lengkap
- Jangan mengubah bagian yang tidak diperlukan
- Ikuti style code proyek
- Test secara lokal dulu
```

### 4. VALIDASI
```
- Pastikan tidak ada breaking change
- Pastikan TypeScript/linter/build tetap bersih
- Jelaskan cara menguji perubahan
```

---

## Prioritas Pengembangan

| Priority | Phase | Deskripsi |
|----------|-------|-----------|
| 1 | PHASE 1 | Kernel API Layer ✅ DONE |
| 2 | PHASE 2 | Native Runtime Providers ✅ DONE |
| 3 | PHASE 3 | Runtime ↔ JowoKernel Integration ✅ DONE |
| 4 | PHASE 4 | Boot Pipeline |
| 5 | PHASE 5 | Window System |
| 6 | PHASE 6 | Filesystem |
| 7 | PHASE 7 | WebKit Integration |
| 8 | PHASE 8 | Desktop Applications |
| 9 | PHASE 9 | Package Manager |
| 10 | PHASE 10 | Hardware Support |
| 11 | PHASE 11 | Optimization |
| 12 | PHASE 12 | Release |

---

## Struktur Direktori

```
klat-os/
├── kernel/                    # JowoKernel C++ code
│   ├── arch/x86/             # x86 architecture
│   ├── subsystems/            # 24 subsystems
│   ├── dev/drivers/          # Hardware drivers
│   ├── vm/                   # Virtual memory
│   └── fs/                   # Filesystem
│
├── kernel-api/               # Kernel API Layer (Phase 1 ✅)
│   ├── contracts/             # Interface definitions
│   ├── dispatcher/           # Syscall dispatcher
│   ├── handles/              # Handle management
│   └── errors/               # Error handling
│
├── userspace/                 # Userspace programs (Phase 3 ✅)
│   ├── libjowo/              # JowoLibc
│   ├── init/                 # Init process
│   └── shell/                # Shell
│
├── klat-desktop/              # KLAT Desktop UI (PROTECTED - STABLE)
│   └── src/                   # Svelte 5 components
│
└── build-x86_64/              # Build output
```

---

## Checklist Sebelum Commit

- [ ] Kode sesuai arsitektur
- [ ] GUI TIDAK berubah
- [ ] TypeScript check: `npx tsc --noEmit` ✅
- [ ] Build: `npm run build` ✅
- [ ] Kernel build: `make kernel` ✅
- [ ] Tidak ada regression
- [ ] Dokumentasi diupdate
- [ ] Test di QEMU (jika applicable)

---

## Hal yang Dilarang

1. ❌ Langsung coding tanpa analisis
2. ❌ Skip phase
3. ❌ Bypass Kernel API Layer
4. ❌ Import browser API di Runtime
5. ❌ Blocking syscall di user space
6. ❌ Hardcoded path (gunakan konstanta)
7. ❌ Commit dengan build error
8. ❌ Breaking change tanpa versioning
9. ❌ Ubah GUI tanpa instruksi eksplisit
10. ❌ Redesign Desktop Experience

---

## Contoh Implementasi yang Benar

### ❌ SALAH (Langsung syscall)
```typescript
// DILARANG
const pid = syscall(1, 'spawn', program);
```

### ✅ BENAR (Lewat Kernel API)
```typescript
// ProcessRuntime.ts
import { KernelAPI } from '$kernel-api';

class ProcessRuntime {
  async spawn(program: string): Promise<ProcessHandle> {
    return KernelAPI.process.spawn(program);
  }
}
```

### ❌ SALAH (Dependency browser)
```typescript
// DILARANG di Runtime
const data = localStorage.getItem('key');
```

### ✅ BENAR (Lewat Provider)
```typescript
// Runtime.ts
import { StorageProvider } from '../providers/storage_provider';

class StorageRuntime {
  private provider: StorageProvider;
  
  async get(key: string): Promise<string | null> {
    return this.provider.get(key);
  }
}
```

### ❌ SALAH (Ubah GUI)
```typescript
// DILARANG - Mengubah DesktopShell.ts
class DesktopShell {
  // ❌ Tidak boleh mengubah ini
  // Hanya boleh menambah jika benar-benar perlu
}
```

### ✅ BENAR (Tambah Provider)
```typescript
// ✅ BENAR - Menambah NativeFilesystemProvider
class NativeFilesystemProvider implements IFilesystemProvider {
  // Ini yang berubah, bukan DesktopShell
  async read(path: string): Promise<Uint8Array> {
    return this.kernel.filesystem.read(path);
  }
}
```

---

## Testing Strategy

### Unit Test
- Runtime modules
- Kernel API
- Provider implementations

### Integration Test
- Runtime ↔ Kernel API
- Native Provider ↔ JowoKernel

### System Test
- Full boot di QEMU
- Desktop shell rendering (GUI tetap sama)
- Window operations
- File operations

### Performance Test
- Boot time < 5 detik
- Memory usage < 512MB
- FPS desktop > 60

---

## Referensi

- [ROADMAP.md](./ROADMAP.md) — Timeline dan milestone
- [PROGRESS.md](./PROGRESS.md) — Progress tracker
- [klat-desktop/](.//klat-desktop/) — Desktop UI (PROTECTED)

---

## Ringkasan Aturan

| Yang Harus | Yang Tidak Boleh |
|------------|------------------|
| Bikin Runtime | Ubah GUI |
| Bikin Kernel API | Redesign Desktop |
| Bikin Native Provider | Rewrite Launcher |
| Hubungkan ke GUI | Bypass Provider |
| Jaga API stabil | Langsung syscall |
| Test di QEMU | Commit dengan error |

---

**GUI tetap.**
**Runtime berkembang.**
**Kernel berkembang.**
**Provider berkembang.**

**Desktop Experience harus tetap konsisten di Browser Emulator maupun Native KLAT OS.**

---

*Prompt ini WAJIB digunakan setiap kali AI bekerja di proyek KLAT OS.*
*Setiap implementasi harus membawa KLAT OS selangkah lebih dekat menuju boot penuh di QEMU.*
*GUI adalah sacred - lindungi selalu.*
