# KLAT OS RFC Index

**Status**: Active  
**Last Updated**: 2026-07-08

---

## About RFCs

RFC (Request for Comments) is the process for proposing, reviewing, and accepting architectural decisions for KLAT OS. All major changes to the system architecture must go through the RFC process.

### RFC Workflow

```
1. Draft    → RFC is being written
     ↓
2. Review   → RFC is open for community feedback
     ↓
3. Accepted → RFC has been approved for implementation
     ↓
4. Implemented → RFC has been implemented
     ↓
5. Rejected → RFC was not accepted
     ↓
6. Superseded → RFC has been replaced by another
```

---

## RFC List

| Number | Title | Status | Date |
|--------|-------|--------|------|
| 0001 | Kernel Syscall ABI Freeze | ✅ Accepted | 2026-07-08 |

---

## Pending RFCs

The following topics are planned for future RFCs:

| Topic | Description | Priority |
|-------|-------------|----------|
| 0002 | Runtime API (crt0, libc) | High |
| 0003 | Window Protocol | High |
| 0004 | Graphics Stack Architecture | Medium |
| 0005 | SDK Widget API | Medium |
| 0006 | Process Lifecycle | Medium |
| 0007 | Memory Management | Low |
| 0008 | Filesystem API | Low |

---

## Submitting an RFC

### 1. Create Draft

Create a new file in `docs/RFC/` following the template:

```bash
cp docs/RFC/TEMPLATE.md docs/RFC/XXXX-topic-slug.md
```

Edit the file with your proposal.

### 2. Submit for Review

Open a pull request with the RFC draft. The title should be:

```
RFC: XXXX - [Title]
```

### 3. Address Feedback

Update the RFC based on community feedback.

### 4. Final Review

Once consensus is reached, change the status to "Accepted".

### 5. Implement

Implement the RFC and update the status to "Implemented".

---

## RFC Template

See `docs/RFC/TEMPLATE.md` for the RFC template.

---

## Guidelines

### Good RFC Topics

- Architectural decisions
- API changes
- New subsystems
- Protocol definitions
- Major refactoring

### Bad RFC Topics

- Bug fixes (use issues)
- Minor documentation changes
- Individual feature requests (use issues)
- Code style (use linters)

---

## Change Log

| Date | Version | Changes |
|------|---------|---------|
| 2026-07-08 | 1.0 | Initial RFC index |

---

**Maintainer**: KLAT OS Team
