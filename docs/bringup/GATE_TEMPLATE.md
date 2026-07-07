# KLAT OS Gate Template

> **Format standar untuk setiap gate**

---

# [GATE_CODE]: [GATE_NAME]

## Header

| Field | Value |
|-------|-------|
| Gate | [GATE_CODE] |
| Name | [GATE_NAME] |
| Milestone | M[MILESTONE] |
| Status | ⚪ PENDING / 🔴 FAIL / ✅ PASS |
| Date Started | [YYYY-MM-DD] |
| Date Completed | [YYYY-MM-DD] |
| Tested By | [Name] |

---

## Requirement

```
[Apa yang harus terjadi]
```

## Target Flow

```
[Komponen A]
    ↓
[Komponen B]
    ↓
[Komponen C]
```

## Test Command

```bash
[Command untuk test gate ini]
```

## Expected Output

```
[Serial output yang diharapkan]
```

## Actual Output

```
[Serial output yang sebenarnya]
```

---

## Definition of Done

- [ ] [Criterion 1]
- [ ] [Criterion 2]
- [ ] [Criterion 3]

## Results

| Check | Expected | Actual | Status |
|-------|-----------|--------|--------|
| [Check 1] | [Expected] | [Actual] | ✅/🔴 |
| [Check 2] | [Expected] | [Actual] | ✅/🔴 |
| [Check 3] | [Expected] | [Actual] | ✅/🔴 |

---

## Evidence

### Screenshot
```
[Screenshot boot/serial output]
```

### Serial Log
```
[Full serial output]
```

### Commit
```
[Kernel commit hash]
```

---

## Issues (if any)

| Issue | Severity | Status |
|-------|----------|--------|
| [Issue 1] | HIGH | FIXED |

---

## Regression Test

```
Gate sebelumnya: [GATE_CODE-1]
Status: [PASS/FAIL]
```

---

## Sign-off

```
Tested By: [Name]
Approved By: [Name]
Date: [YYYY-MM-DD]
```

---

# Template Checklist

```
[ ] Isi Header dengan informasi gate
[ ] Tulis Requirement
[ ] Gambar Target Flow
[ ] Cantum Test Command
[ ] Capture Expected Output
[ ] Capture Actual Output
[ ] Checklist DoD
[ ] Isi Results table
[ ] Tambah Evidence (screenshot/log/commit)
[ ] Catat Issues jika ada
[ ] Regression test gate sebelumnya
[ ] Sign-off
```

---

*Use this template for every gate M1.1-M5.5*
