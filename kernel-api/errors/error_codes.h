// Copyright (c) 2026 KLAT OS Team
// SPDX-License-Identifier: MIT
//
// JowoKernel Error Codes
// Compatible with Linux errno.h and JowoKernel specific codes

#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================
// Linux-compatible errno values (used by syscalls)
// ============================================================
#define KERNEL_EPERM          1   // Operation not permitted
#define KERNEL_ENOENT         2   // No such file or directory
#define KERNEL ESRCH          3   // No such process
#define KERNEL_EINTR          4   // Interrupted system call
#define KERNEL_EIO            5   // I/O error
#define KERNEL_ENXIO          6   // No such device or address
#define KERNEL_E2BIG          7   // Argument list too long
#define KERNEL_ENOEXEC        8   // Exec format error
#define KERNEL_EBADF          9   // Bad file number
#define KERNEL_ECHILD         10  // No child processes
#define KERNEL_EAGAIN         11  // Try again
#define KERNEL_ENOMEM         12  // Out of memory
#define KERNEL_EACCES         13  // Permission denied
#define KERNEL_EFAULT         14  // Bad address
#define KERNEL_ENOTBLK        15  // Block device required
#define KERNEL_EBUSY          16  // Device or resource busy
#define KERNEL_EEXIST         17  // File exists
#define KERNEL_EXDEV          18  // Cross-device link
#define KERNEL_ENODEV         19  // No such device
#define KERNEL_ENOTDIR        20  // Not a directory
#define KERNEL_EISDIR         21  // Is a directory
#define KERNEL_EINVAL         22  // Invalid argument
#define KERNEL_ENFILE         23  // File table overflow
#define KERNEL_EMFILE         24  // Too many open files
#define KERNEL_ENOTTY         25  // Not a typewriter
#define KERNEL_ETXTBSY        26  // Text file busy
#define KERNEL_EFBIG          27  // File too large
#define KERNEL_ENOSPC         28  // No space left on device
#define KERNEL_ESPIPE         29  // Illegal seek
#define KERNEL_EROFS          30  // Read-only file system
#define KERNEL_EMLINK         31  // Too many links
#define KERNEL_EPIPE          32  // Broken pipe
#define KERNEL_EDOM           33  // Math argument out of domain
#define KERNEL_ERANGE         34  // Math result not representable
#define KERNEL_EDEADLK        35  // Resource deadlock would occur
#define KERNEL_ENAMETOOLONG   36  // File name too long
#define KERNEL_ENOLCK         37  // No record locks available
#define KERNEL_ENOSYS         38  // Function not implemented
#define KERNEL_ENOTEMPTY      39  // Directory not empty
#define KERNEL_ELOOP          40  // Too many symbolic links encountered
#define KERNEL_ENOMSG         42  // No message of desired type
#define KERNEL_EIDRM          43  // Identifier removed
#define KERNEL_ECHRNG         44  // Channel number out of range
#define KERNEL_EL2NSYNC       45  // Level 2 not synchronized
#define KERNEL_EL3HLT         46  // Level 3 halted
#define KERNEL_EL3RST         47  // Level 3 reset
#define KERNEL_ELNRNG         48  // Link number out of range
#define KERNEL_EUNATCH        49  // Protocol driver not attached
#define KERNEL_ENOCSI        50  // No CSI structure available
#define KERNEL_EL2HLT         51  // Level 2 halted
#define KERNEL_EBADE          52  // Invalid exchange
#define KERNEL_EBADR          53  // Invalid request descriptor
#define KERNEL_EXFULL         54  // Exchange full
#define KERNEL_ENOANO         55  // No anode
#define KERNEL_EBADRQC        56  // Invalid request code
#define KERNEL_EBADSLT        57  // Invalid slot
#define KERNEL_EBFONT         59  // Bad font file format
#define KERNEL_ENOSTR         60  // Device not a stream
#define KERNEL_ENODATA        61  // No data available
#define KERNEL_ETIME          62  // Timer expired
#define KERNEL_ENOSR          63  // Out of streams resources
#define KERNEL_ENONET         64  // Machine is not on the network
#define KERNEL_ENOPKG         65  // Package not installed
#define KERNEL_EREMOTE        66  // Object is remote
#define KERNEL_ENOLINK        67  // Link has been severed
#define KERNEL_EADV           68  // Advertise error
#define KERNEL_ESRMNT         69  // Srmount error
#define KERNEL_ECOMM          70  // Communication error on send
#define KERNEL_EPROTO         71  // Protocol error
#define KERNEL_EMULTIHOP      72  // Multihop attempted
#define KERNEL_EDOTDOT        73  // RFS specific error
#define KERNEL_EBADMSG        74  // Not a data message
#define KERNEL_EOVERFLOW      75  // Value too large for defined data type
#define KERNEL_ENOTUNIQ       76  // Name not unique on network
#define KERNEL_EBADFD         77  // File descriptor in bad state
#define KERNEL_EREMCHG        78  // Remote address changed
#define KERNEL_ELIBACC        79  // Can not access a needed shared library
#define KERNEL_ELIBBAD        80  // Accessing a corrupted shared library
#define KERNEL_ELIBSCN        81  // .lib section in a.out corrupted
#define KERNEL_ELIBMAX        82  // Attempting to link in too many shared libraries
#define KERNEL_ELIBEXEC       83  // Cannot exec a shared library directly
#define KERNEL_EILSEQ         84  // Illegal byte sequence
#define KERNEL_ERESTART       85  // Interrupted system call should be restarted
#define KERNEL_ESTRPIPE       86  // Streams pipe error
#define KERNEL_EUSERS         87  // Too many users
#define KERNEL_ENOTSOCK       88  // Socket operation on non-socket
#define KERNEL_EDESTADDRREQ   89  // Destination address required
#define KERNEL_EMSGSIZE       90  // Message too long
#define KERNEL_EPROTOTYPE     91  // Protocol wrong type for socket
#define KERNEL_ENOPROTOOPT    92  // Protocol not available
#define KERNEL_EPROTONOSUPPORT 93 // Protocol not supported
#define KERNEL_ESOCKTNOSUPPORT 94 // Socket type not supported
#define KERNEL_EOPNOTSUPP     95  // Operation not supported on transport endpoint
#define KERNEL_EPFNOSUPPORT   96  // Protocol family not supported
#define KERNEL_EAFNOSUPPORT   97  // Address family not supported by protocol
#define KERNEL_EADDRINUSE     98  // Address already in use
#define KERNEL_EADDRNOTAVAIL  99  // Cannot assign requested address
#define KERNEL_ENETDOWN       100 // Network is down
#define KERNEL_ENETUNREACH     101 // Network is unreachable
#define KERNEL_ENETRESET      102 // Network dropped connection because of reset
#define KERNEL_ECONNABORTED    103 // Software caused connection abort
#define KERNEL_ECONNRESET      104 // Connection reset by peer
#define KERNEL_ENOBUFS         105 // No buffer space available
#define KERNEL_EISCONN         106 // Transport endpoint is already connected
#define KERNEL_ENOTCONN        107 // Transport endpoint is not connected
#define KERNEL_ESHUTDOWN       108 // Cannot send after transport endpoint shutdown
#define KERNEL_ETOOMANYREFS    109 // Too many references: cannot splice
#define KERNEL_ETIMEDOUT       110 // Connection timed out
#define KERNEL_ECONNREFUSED    111 // Connection refused
#define KERNEL_EHOSTDOWN       112 // Host is down
#define KERNEL_EHOSTUNREACH    113 // No route to host
#define KERNEL_EALREADY        114 // Operation already in progress
#define KERNEL_EINPROGRESS     115 // Operation now in progress
#define KERNEL_ESTALE          116 // Stale NFS file handle
#define KERNEL_EUCLEAN         117 // Structure needs cleaning
#define KERNEL_ENOTNAM         118 // Not a XENIX named type of file
#define KERNEL_ENAVAIL         119 // No XENIX semaphores available
#define KERNEL_EISNAM          120 // Is a named type file
#define KERNEL_EREMOTEIO       121 // Remote I/O error
#define KERNEL_EDQUOT          122 // Quota exceeded
#define KERNEL_ENOMEDIUM       123 // No medium found
#define KERNEL_EMEDIUMTYPE     124 // Wrong medium type
#define KERNEL_ECANCELED       125 // Operation Canceled
#define KERNEL_ENOKEY          126 // Required key not available
#define KERNEL_EKEYEXPIRED     127 // Key has expired
#define KERNEL_EKEYREVOKED     128 // Key has been revoked
#define KERNEL_EKEYREJECTED    129 // Key was rejected by service
#define KERNEL_EOWNERDEAD      130 // Owner died
#define KERNEL_ENOTRECOVERABLE 131 // State not recoverable
#define KERNEL_ERFKILL         132 // Operation not possible due to RF-kill
#define KERNEL_EHWPOISON       133 // Memory page has hardware error

// ============================================================
// JowoKernel-specific error codes (starting from 1000)
// ============================================================
#define KERNEL_EHANDLE         1000  // Invalid handle
#define KERNEL_EHANDLECLOSED   1001  // Handle already closed
#define KERNEL_ERIGHTS         1002  // Insufficient rights
#define KERNEL_EOBJECTWAIT     1003  // Object wait timeout
#define KERNEL_EIPC            1004  // IPC error
#define KERNEL_EMEMORY         1005  // Memory allocation failed
#define KERNEL_EVMO            1006  // VMO operation failed
#define KERNEL_ETHREAD         1007  // Thread error
#define KERNEL_EPROCESS        1008  // Process error
#define KERNEL_EVFS            1009  // VFS operation failed
#define KERNEL_EDRIVER         1010  // Driver error
#define KERNEL_EINTERRUPT      1011  // Interrupt error
#define KERNEL_EPCI            1012  // PCI error
#define KERNEL_EGPU            1013  // GPU error
#define KERNEL_ENOTREADY       1014  // Resource not ready
#define KERNEL_EALIGNMENT       1015 // Memory alignment error

// ============================================================
// Success indicator
// ============================================================
#define KERNEL_SUCCESS         0

#ifdef __cplusplus
}
#endif
