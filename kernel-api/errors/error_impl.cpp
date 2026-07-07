// Copyright (c) 2026 KLAT OS Team
// SPDX-License-Identifier: MIT
//
// JowoKernel API Implementation

#include "kernel_api.h"
#include "errors/error_codes.h"

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================
// Error String Lookup Table
// ============================================================

typedef struct {
    int code;
    const char* message;
} error_entry_t;

static const error_entry_t error_table[] = {
    { KERNEL_EPERM,           "Operation not permitted" },
    { KERNEL_ENOENT,          "No such file or directory" },
    { KERNEL_ESRCH,           "No such process" },
    { KERNEL_EINTR,           "Interrupted system call" },
    { KERNEL_EIO,             "I/O error" },
    { KERNEL_ENXIO,           "No such device or address" },
    { KERNEL_E2BIG,           "Argument list too long" },
    { KERNEL_ENOEXEC,         "Exec format error" },
    { KERNEL_EBADF,           "Bad file number" },
    { KERNEL_ECHILD,          "No child processes" },
    { KERNEL_EAGAIN,          "Try again" },
    { KERNEL_ENOMEM,          "Out of memory" },
    { KERNEL_EACCES,          "Permission denied" },
    { KERNEL_EFAULT,          "Bad address" },
    { KERNEL_ENOTBLK,         "Block device required" },
    { KERNEL_EBUSY,           "Device or resource busy" },
    { KERNEL_EEXIST,          "File exists" },
    { KERNEL_EXDEV,           "Cross-device link" },
    { KERNEL_ENODEV,          "No such device" },
    { KERNEL_ENOTDIR,         "Not a directory" },
    { KERNEL_EISDIR,          "Is a directory" },
    { KERNEL_EINVAL,          "Invalid argument" },
    { KERNEL_ENFILE,          "File table overflow" },
    { KERNEL_EMFILE,          "Too many open files" },
    { KERNEL_ENOTTY,          "Not a typewriter" },
    { KERNEL_ETXTBSY,         "Text file busy" },
    { KERNEL_EFBIG,           "File too large" },
    { KERNEL_ENOSPC,          "No space left on device" },
    { KERNEL_ESPIPE,          "Illegal seek" },
    { KERNEL_EROFS,           "Read-only file system" },
    { KERNEL_EMLINK,          "Too many links" },
    { KERNEL_EPIPE,           "Broken pipe" },
    { KERNEL_EDOM,            "Math argument out of domain" },
    { KERNEL_ERANGE,          "Math result not representable" },
    { KERNEL_EDEADLK,         "Resource deadlock would occur" },
    { KERNEL_ENAMETOOLONG,    "File name too long" },
    { KERNEL_ENOLCK,          "No record locks available" },
    { KERNEL_ENOSYS,          "Function not implemented" },
    { KERNEL_ENOTEMPTY,       "Directory not empty" },
    { KERNEL_ELOOP,           "Too many symbolic links encountered" },
    { KERNEL_EWOULDBLOCK,     "Operation would block" },
    { KERNEL_ENOMSG,          "No message of desired type" },
    { KERNEL_EIDRM,           "Identifier removed" },
    { KERNEL_ECHRNG,          "Channel number out of range" },
    { KERNEL_EL2NSYNC,        "Level 2 not synchronized" },
    { KERNEL_EL3HLT,          "Level 3 halted" },
    { KERNEL_EL3RST,          "Level 3 reset" },
    { KERNEL_ELNRNG,          "Link number out of range" },
    { KERNEL_EUNATCH,         "Protocol driver not attached" },
    { KERNEL_ENOCSI,          "No CSI structure available" },
    { KERNEL_EL2HLT,          "Level 2 halted" },
    { KERNEL_EBADE,           "Invalid exchange" },
    { KERNEL_EBADR,           "Invalid request descriptor" },
    { KERNEL_EXFULL,          "Exchange full" },
    { KERNEL_ENOANO,          "No anode" },
    { KERNEL_EBADRQC,         "Invalid request code" },
    { KERNEL_EBADSLT,         "Invalid slot" },
    { KERNEL_EBFONT,          "Bad font file format" },
    { KERNEL_ENOSTR,          "Device not a stream" },
    { KERNEL_ENODATA,         "No data available" },
    { KERNEL_ETIME,           "Timer expired" },
    { KERNEL_ENOSR,           "Out of streams resources" },
    { KERNEL_ENONET,          "Machine is not on the network" },
    { KERNEL_ENOPKG,          "Package not installed" },
    { KERNEL_EREMOTE,         "Object is remote" },
    { KERNEL_ENOLINK,         "Link has been severed" },
    { KERNEL_EADV,            "Advertise error" },
    { KERNEL_ESRMNT,          "Srmount error" },
    { KERNEL_ECOMM,           "Communication error on send" },
    { KERNEL_EPROTO,          "Protocol error" },
    { KERNEL_EMULTIHOP,       "Multihop attempted" },
    { KERNEL_EDOTDOT,         "RFS specific error" },
    { KERNEL_EBADMSG,         "Not a data message" },
    { KERNEL_EOVERFLOW,       "Value too large for defined data type" },
    { KERNEL_ENOTUNIQ,        "Name not unique on network" },
    { KERNEL_EBADFD,          "File descriptor in bad state" },
    { KERNEL_EREMCHG,         "Remote address changed" },
    { KERNEL_ELIBACC,         "Cannot access a needed shared library" },
    { KERNEL_ELIBBAD,         "Accessing a corrupted shared library" },
    { KERNEL_ELIBSCN,         ".lib section in a.out corrupted" },
    { KERNEL_ELIBMAX,         "Attempting to link in too many shared libraries" },
    { KERNEL_ELIBEXEC,        "Cannot exec a shared library directly" },
    { KERNEL_EILSEQ,          "Illegal byte sequence" },
    { KERNEL_ERESTART,        "Interrupted system call should be restarted" },
    { KERNEL_ESTRPIPE,        "Streams pipe error" },
    { KERNEL_EUSERS,          "Too many users" },
    { KERNEL_ENOTSOCK,        "Socket operation on non-socket" },
    { KERNEL_EDESTADDRREQ,    "Destination address required" },
    { KERNEL_EMSGSIZE,        "Message too long" },
    { KERNEL_EPROTOTYPE,      "Protocol wrong type for socket" },
    { KERNEL_ENOPROTOOPT,     "Protocol not available" },
    { KERNEL_EPROTONOSUPPORT, "Protocol not supported" },
    { KERNEL_ESOCKTNOSUPPORT, "Socket type not supported" },
    { KERNEL_EOPNOTSUPP,      "Operation not supported on transport endpoint" },
    { KERNEL_EPFNOSUPPORT,    "Protocol family not supported" },
    { KERNEL_EAFNOSUPPORT,    "Address family not supported by protocol" },
    { KERNEL_EADDRINUSE,      "Address already in use" },
    { KERNEL_EADDRNOTAVAIL,   "Cannot assign requested address" },
    { KERNEL_ENETDOWN,        "Network is down" },
    { KERNEL_ENETUNREACH,     "Network is unreachable" },
    { KERNEL_ENETRESET,       "Network dropped connection because of reset" },
    { KERNEL_ECONNABORTED,    "Software caused connection abort" },
    { KERNEL_ECONNRESET,      "Connection reset by peer" },
    { KERNEL_ENOBUFS,         "No buffer space available" },
    { KERNEL_EISCONN,         "Transport endpoint is already connected" },
    { KERNEL_ENOTCONN,        "Transport endpoint is not connected" },
    { KERNEL_ESHUTDOWN,       "Cannot send after transport endpoint shutdown" },
    { KERNEL_ETOOMANYREFS,    "Too many references: cannot splice" },
    { KERNEL_ETIMEDOUT,       "Connection timed out" },
    { KERNEL_ECONNREFUSED,    "Connection refused" },
    { KERNEL_EHOSTDOWN,       "Host is down" },
    { KERNEL_EHOSTUNREACH,    "No route to host" },
    { KERNEL_EALREADY,        "Operation already in progress" },
    { KERNEL_EINPROGRESS,     "Operation now in progress" },
    { KERNEL_ESTALE,          "Stale NFS file handle" },
    { KERNEL_EUCLEAN,         "Structure needs cleaning" },
    { KERNEL_ENOTNAM,         "Not a XENIX named type of file" },
    { KERNEL_ENAVAIL,         "No XENIX semaphores available" },
    { KERNEL_EISNAM,          "Is a named type file" },
    { KERNEL_EREMOTEIO,       "Remote I/O error" },
    { KERNEL_EDQUOT,          "Quota exceeded" },
    { KERNEL_ENOMEDIUM,       "No medium found" },
    { KERNEL_EMEDIUMTYPE,     "Wrong medium type" },
    { KERNEL_ECANCELED,       "Operation Canceled" },
    { KERNEL_ENOKEY,          "Required key not available" },
    { KERNEL_EKEYEXPIRED,     "Key has expired" },
    { KERNEL_EKEYREVOKED,     "Key has been revoked" },
    { KERNEL_EKEYREJECTED,    "Key was rejected by service" },
    { KERNEL_EOWNERDEAD,      "Owner died" },
    { KERNEL_ENOTRECOVERABLE, "State not recoverable" },
    { KERNEL_ERFKILL,         "Operation not possible due to RF-kill" },
    { KERNEL_EHWPOISON,       "Memory page has hardware error" },

    // JowoKernel-specific errors
    { KERNEL_EHANDLE,         "Invalid handle" },
    { KERNEL_EHANDLECLOSED,   "Handle already closed" },
    { KERNEL_ERIGHTS,         "Insufficient rights" },
    { KERNEL_EOBJECTWAIT,     "Object wait timeout" },
    { KERNEL_EIPC,            "IPC error" },
    { KERNEL_EMEMORY,         "Memory allocation failed" },
    { KERNEL_EVMO,            "VMO operation failed" },
    { KERNEL_ETHREAD,         "Thread error" },
    { KERNEL_EPROCESS,        "Process error" },
    { KERNEL_EVFS,            "VFS operation failed" },
    { KERNEL_EDRIVER,         "Driver error" },
    { KERNEL_EINTERRUPT,      "Interrupt error" },
    { KERNEL_EPCI,            "PCI error" },
    { KERNEL_EGPU,            "GPU error" },
    { KERNEL_ENOTREADY,       "Resource not ready" },
    { KERNEL_EALIGNMENT,      "Memory alignment error" },

    { 0, NULL }
};

// ============================================================
// Error String Implementation
// ============================================================

const char* kernel_strerror(int err) {
    if (err < 0) {
        err = -err;  // Handle negative errno
    }

    // Handle special case
    if (err == 0) {
        return "Success";
    }

    // Search table
    for (int i = 0; error_table[i].message != NULL; i++) {
        if (error_table[i].code == err) {
            return error_table[i].message;
        }
    }

    return "Unknown error";
}

#ifdef __cplusplus
}
#endif
