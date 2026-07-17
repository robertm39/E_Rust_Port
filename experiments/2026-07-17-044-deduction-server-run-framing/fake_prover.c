#include <stdio.h>
#include <unistd.h>

int main(void)
{
    printf("%%%% Pid: %ld\n", (long)getpid());
    puts("% SZS status Theorem");
    puts("% deterministic reference output");
    return 0;
}
