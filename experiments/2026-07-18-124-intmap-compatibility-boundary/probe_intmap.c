#include <stdint.h>
#include <stdio.h>

#include "clb_intmap.h"

static void* value(long value)
{
   return (void*)(uintptr_t)value;
}

static IntMap_p dense_map(long first)
{
   IntMap_p map = IntMapAlloc();

   IntMapAssign(map, first, value(1));
   IntMapAssign(map, first + 1, value(2));
   return map;
}

static void print_shape(const char* name, IntMap_p map)
{
   long offset = -1;
   long size = -1;

   if(map->type == IMArray)
   {
      offset = map->values.array->offset;
      size = map->values.array->size;
   }
   printf("record=shape,name=%s,type=%d,entries=%lu,min=%ld,max=%ld,"
          "offset=%ld,size=%ld,storage=%zu\n",
          name, map->type, map->entry_no, map->min_key, map->max_key,
          offset, size, (size_t)IntMapStorage(map));
}

static void probe_shapes(void)
{
   IntMap_p map;

   map = IntMapAlloc();
   IntMapAssign(map, 0, value(1));
   print_shape("single", map);
   IntMapFree(map);

   map = dense_map(0);
   print_shape("dense", map);
   IntMapFree(map);

   map = IntMapAlloc();
   IntMapAssign(map, 100, value(1));
   IntMapAssign(map, 0, value(2));
   print_shape("sparse_descending", map);
   IntMapFree(map);

   map = IntMapAlloc();
   IntMapAssign(map, 0, value(1));
   IntMapAssign(map, 100, value(2));
   print_shape("sparse_ascending", map);
   IntMapFree(map);
}

static void probe_null_count(void)
{
   IntMap_p map = IntMapAlloc();
   void** slot;
   unsigned long first;
   unsigned long second;
   unsigned long third;

   IntMapAssign(map, 0, value(1));
   slot = IntMapGetRef(map, 1);
   first = map->entry_no;
   slot = IntMapGetRef(map, 1);
   second = map->entry_no;
   slot = IntMapGetRef(map, 1);
   third = map->entry_no;
   printf("record=null_count,first=%lu,second=%lu,third=%lu,"
          "slot_is_null=%d,type=%d,offset=%ld,size=%ld,storage=%zu\n",
          first, second, third, *slot == NULL, map->type,
          map->values.array->offset, map->values.array->size,
          (size_t)IntMapStorage(map));
   IntMapFree(map);
}

static void print_miss(const char* name, IntMap_p map, int found,
                       long before_offset, long before_size,
                       size_t before_storage)
{
   printf("record=miss,name=%s,found=%d,before_offset=%ld,before_size=%ld,"
          "after_offset=%ld,after_size=%ld,entries=%lu,min=%ld,max=%ld,"
          "before_storage=%zu,after_storage=%zu\n",
          name, found, before_offset, before_size,
          map->values.array->offset, map->values.array->size,
          map->entry_no, map->min_key, map->max_key,
          before_storage, (size_t)IntMapStorage(map));
}

static void probe_misses(void)
{
   IntMap_p map;
   IntMapIter_p iter;
   void* found;
   long key = -1;
   long before_offset;
   long before_size;
   size_t before_storage;

   map = dense_map(10);
   before_offset = map->values.array->offset;
   before_size = map->values.array->size;
   before_storage = (size_t)IntMapStorage(map);
   found = IntMapGetVal(map, 9);
   print_miss("get", map, found != NULL, before_offset, before_size,
              before_storage);
   IntMapFree(map);

   map = dense_map(10);
   before_offset = map->values.array->offset;
   before_size = map->values.array->size;
   before_storage = (size_t)IntMapStorage(map);
   found = IntMapDelKey(map, 9);
   print_miss("delete", map, found != NULL, before_offset, before_size,
              before_storage);
   IntMapFree(map);

   map = dense_map(10);
   before_offset = map->values.array->offset;
   before_size = map->values.array->size;
   before_storage = (size_t)IntMapStorage(map);
   iter = IntMapIterAlloc(map, 9, 11);
   found = IntMapIterNext(iter, &key);
   printf("record=iterator,found=%d,key=%ld,before_offset=%ld,before_size=%ld,"
          "after_offset=%ld,after_size=%ld,entries=%lu,min=%ld,max=%ld,"
          "before_storage=%zu,after_storage=%zu\n",
          found != NULL, key, before_offset, before_size,
          map->values.array->offset, map->values.array->size,
          map->entry_no, map->min_key, map->max_key,
          before_storage, (size_t)IntMapStorage(map));
   IntMapIterFree(iter);
   IntMapFree(map);
}

int main(void)
{
   probe_shapes();
   probe_null_count();
   probe_misses();
   return 0;
}
