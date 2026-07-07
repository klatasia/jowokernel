#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <jowo/syscalls.h>
#include <signal.h>

volatile int sigint_received = 0;

void sigint_handler(int signum) {
    printf("\n[User Handler] Menerima sinyal %d (SIGINT)!\n", signum);
    sigint_received = 1;
}

extern "C" int main() {
    printf("\n=========================================\n");
    printf("        JowoKernel Signal Delivery Test    \n");
    printf("=========================================\n\n");

    printf("[Userspace] Mendaftarkan handler untuk SIGINT (Signal 2)...\n");
    struct sigaction sa;
    sa.sa_handler = sigint_handler;
    sa.sa_flags = 0;
    sa.sa_mask = 0;
    sa.sa_restorer = NULL;
    
    if (sigaction(SIGINT, &sa, NULL) < 0) {
        printf("[Userspace] GAGAL mendaftarkan handler!\n");
        return 1;
    }

    printf("[Userspace] Menunggu dalam infinite loop...\n");
    
    int counter = 0;
    while(1) {
        if (sigint_received) {
            printf("[Userspace] Terdeteksi flag sigint_received dari handler!\n");
            printf("[Userspace] Program kembali ke normal loop! Test SUKSES.\n");
            sigint_received = 0;
            
            // Kita coba sys_kill ke diri sendiri dengan SIGTERM untuk tes stress!
            printf("[Userspace] Menguji SIGTERM (akan terminate otomatis)...\n");
            int my_pid = sys_getpid();
            sys_kill(my_pid, SIGTERM);
        }
        
        counter++;
        if (counter == 200000) {
            printf("[Userspace] Memicu sys_kill(SIGINT) ke diri sendiri...\n");
            sys_kill(sys_getpid(), SIGINT);
        }
        
        if (counter % 100000 == 0) {
            printf("[Userspace] Loop berjalan dengan normal...\n");
        }
        
        sys_yield(); // Beri nafas pada scheduler
    }

    return 0;
}
