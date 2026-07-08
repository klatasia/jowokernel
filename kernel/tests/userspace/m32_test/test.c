void _start() {
    volatile int x = 0;
    while(x < 100000000) { x++; }
    volatile int* p = (volatile int*)0xFFFF800000000000;
    *p = 1;
    while(1) {}
}
