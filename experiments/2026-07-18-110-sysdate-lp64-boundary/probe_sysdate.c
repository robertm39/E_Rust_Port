#include "clb_sysdate.h"

#include <limits.h>
#include <stdio.h>

static void print_date(const char *name, SysDate date)
{
   printf("date.%s=", name);
   SysDatePrint(stdout, date);
   putchar('\n');
}

int main(void)
{
   printf("abi.long_bytes=%zu\n", sizeof(long));
   printf("abi.long_bits=%zu\n", sizeof(long) * CHAR_BIT);
   printf("abi.long_max=%ld\n", LONG_MAX);
   printf("abi.ulong_max=%lu\n", ULONG_MAX);
   print_date("creation", SysDateCreationTime());
   print_date("ordinary", (SysDate)42);
   print_date("invalid", SysDateInvalidTime());
   print_date("maximum", (SysDate)LONG_MAX);
   return 0;
}
