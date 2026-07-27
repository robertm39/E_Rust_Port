#include <stdio.h>
#include <stdlib.h>

#include "clb_memory.h"

static long list_length(long bucket)
{
   long count = 0;
   Mem_p current;

   if(bucket < 0 || bucket >= MEM_ARR_SIZE)
   {
      return -1;
   }
   current = free_mem_list[bucket];
   while(current)
   {
      count++;
      current = current->next;
   }
   return count;
}

int main(int argc, char** argv)
{
   long request;
   long effective;
   long bucket;
   long after_alloc;
   long after_free;
   long after_flush;
   void* block;

   if(argc != 2)
   {
      return 2;
   }
   request = strtol(argv[1], NULL, 10);
   effective = request < (long)sizeof(MemCell) ? (long)sizeof(MemCell) : request;

#ifdef USE_NEWMEM
   bucket = (effective + MEM_ALIGN - 1) / MEM_ALIGN;
   if(bucket >= MEM_ARR_SIZE)
   {
      bucket = -1;
   }
#else
   bucket = request >= (long)sizeof(MemCell) && request < MEM_ARR_SIZE
            ? request : -1;
#endif

   block = SizeMalloc((size_t)request);
   if(request > 0)
   {
      ((unsigned char*)block)[0] = 0x5a;
   }
   after_alloc = list_length(bucket);
   SizeFree(block, (size_t)request);
   after_free = list_length(bucket);
   MemFlushFreeList();
   after_flush = list_length(bucket);

#ifdef USE_NEWMEM
   printf("mode=new,request=%ld,min=%zu,align=%d,chunk_limit=%d,multiplier=%d,"
          "bucket=%ld,after_alloc=%ld,after_free=%ld,after_flush=%ld\n",
          request, sizeof(MemCell), MEM_ALIGN, MEM_CHUNKLIMIT, MEM_MULTIPLIER,
          bucket, after_alloc, after_free, after_flush);
#else
   printf("mode=old,request=%ld,min=%zu,align=0,chunk_limit=0,multiplier=0,"
          "bucket=%ld,after_alloc=%ld,after_free=%ld,after_flush=%ld\n",
          request, sizeof(MemCell), bucket, after_alloc, after_free, after_flush);
#endif
   return 0;
}
