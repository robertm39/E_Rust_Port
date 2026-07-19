#include <stdint.h>
#include <stdio.h>

#include "clb_plocalstacks.h"

int main(void)
{
   long objects[40];
   unsigned long long checksum = 0;
   PLocalTaggedStackInit(stack);

   PLocalTaggedStackEnsureSpace(stack, 40);
   for(long i=0; i<40; i++)
   {
      objects[i] = i;
      PLocalTaggedStackPush(stack, &objects[i], (uintptr_t)(i&3));
   }

   const size_t current = stack_current;
   const size_t size = stack_size;
   for(long expected=39; expected>=0; expected--)
   {
      long* value;
      uintptr_t tag;

      PLocalTaggedStackPop(stack, value, tag);
      if(value != &objects[expected] || tag != (uintptr_t)(expected&3))
      {
         return 2;
      }
      checksum += (unsigned long long)(expected+1)*5 + tag;
   }

#ifdef TAGGED_POINTERS
   const char* mode = "tagged";
#else
   const char* mode = "portable";
#endif
   printf("mode=%s,pointer_bytes=%zu,tag_bits=%d,tag_mask=%lu,size=%zu,current=%zu,"
          "entry_slots=%zu,allocated_bytes=%zu,checksum=%llu\n",
          mode,
          sizeof(void*),
          PLOCALSTACK_TAG_BITS,
          (unsigned long)PLOCALSTACK_TAG_MASK,
          size,
          current,
          current/40,
          size*sizeof(void*),
          checksum);

   PLocalTaggedStackFree(stack);
   return 0;
}
