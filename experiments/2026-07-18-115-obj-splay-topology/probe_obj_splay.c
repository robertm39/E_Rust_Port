#include <clb_objmaps.h>
#include <clb_objtrees.h>

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>

struct objmap_node
{
   struct objmap_node *lson;
   struct objmap_node *rson;
   void* key;
   void* value;
};

static int int_compare(const void* left, const void* right)
{
   intptr_t left_value = (intptr_t)left;
   intptr_t right_value = (intptr_t)right;
   return (left_value > right_value) - (left_value < right_value);
}

static void print_obj_tree(PObjTree_p tree)
{
   if(!tree)
   {
      putchar('.');
      return;
   }
   printf("[%ld](", (long)(intptr_t)tree->key);
   print_obj_tree(tree->lson);
   putchar(',');
   print_obj_tree(tree->rson);
   putchar(')');
}

static void print_obj_map(PObjMap_p map)
{
   if(!map)
   {
      putchar('.');
      return;
   }
   printf("[%ld](", (long)(intptr_t)map->key);
   print_obj_map(map->lson);
   putchar(',');
   print_obj_map(map->rson);
   putchar(')');
}

static void print_tree_line(const char* label, PObjTree_p tree)
{
   printf("tree %s=", label);
   print_obj_tree(tree);
   putchar('\n');
}

static void print_map_line(const char* label, PObjMap_p map)
{
   printf("map %s=", label);
   print_obj_map(map);
   putchar('\n');
}

static bool first_deleted;

static void print_tree_delete(void* object)
{
   printf("%s%ld", first_deleted ? "" : ",", (long)(intptr_t)object);
   first_deleted = false;
}

static void print_map_delete(void* key, void* value)
{
   (void)value;
   printf("%s%ld", first_deleted ? "" : ",", (long)(intptr_t)key);
   first_deleted = false;
}

int main(void)
{
   PObjTree_p tree = NULL;
   print_tree_line("empty", tree);
   PTreeObjStore(&tree, (void*)(intptr_t)4, int_compare);
   print_tree_line("store4", tree);
   PTreeObjStore(&tree, (void*)(intptr_t)2, int_compare);
   print_tree_line("store2", tree);
   PTreeObjStore(&tree, (void*)(intptr_t)6, int_compare);
   print_tree_line("store6", tree);
   PTreeObjStore(&tree, (void*)(intptr_t)3, int_compare);
   print_tree_line("store3", tree);
   PTreeObjStore(&tree, (void*)(intptr_t)4, int_compare);
   print_tree_line("duplicate4", tree);
   PTreeObjFind(&tree, (void*)(intptr_t)2, int_compare);
   print_tree_line("find2", tree);
   PTreeObjFindBinary(tree, (void*)(intptr_t)6, int_compare);
   print_tree_line("binary6", tree);
   PTreeObjFind(&tree, (void*)(intptr_t)1, int_compare);
   print_tree_line("miss1", tree);
   PTreeObjFind(&tree, (void*)(intptr_t)9, int_compare);
   print_tree_line("miss9", tree);
   PTreeObjFind(&tree, (void*)(intptr_t)4, int_compare);
   print_tree_line("find4", tree);
   PTreeObjExtractObject(&tree, (void*)(intptr_t)5, int_compare);
   print_tree_line("extract_miss5", tree);
   PTreeObjExtractObject(&tree, (void*)(intptr_t)3, int_compare);
   print_tree_line("extract3", tree);
   PTreeObjExtractRootObject(&tree, int_compare);
   print_tree_line("extract_root", tree);
   PObjTreeFree(tree, DummyObjDelFun);

   PObjTree_p base = NULL;
   PObjTree_p add = NULL;
   PTreeObjStore(&base, (void*)(intptr_t)1, int_compare);
   PTreeObjStore(&base, (void*)(intptr_t)3, int_compare);
   PTreeObjStore(&add, (void*)(intptr_t)2, int_compare);
   PTreeObjStore(&add, (void*)(intptr_t)4, int_compare);
   PTreeObjMerge(&base, add, int_compare);
   print_tree_line("merge", base);
   PObjTreeFree(base, DummyObjDelFun);

   tree = NULL;
   PTreeObjStore(&tree, (void*)(intptr_t)3, int_compare);
   PTreeObjStore(&tree, (void*)(intptr_t)1, int_compare);
   PTreeObjStore(&tree, (void*)(intptr_t)2, int_compare);
   printf("tree free=");
   first_deleted = true;
   PObjTreeFree(tree, print_tree_delete);
   putchar('\n');

   PObjMap_p map = NULL;
   print_map_line("empty", map);
   PObjMapStore(&map, (void*)(intptr_t)4, (void*)(intptr_t)40, int_compare);
   print_map_line("store4", map);
   PObjMapStore(&map, (void*)(intptr_t)2, (void*)(intptr_t)20, int_compare);
   print_map_line("store2", map);
   PObjMapStore(&map, (void*)(intptr_t)6, (void*)(intptr_t)60, int_compare);
   print_map_line("store6", map);
   PObjMapStore(&map, (void*)(intptr_t)3, (void*)(intptr_t)30, int_compare);
   print_map_line("store3", map);
   PObjMapStore(&map, (void*)(intptr_t)4, (void*)(intptr_t)44, int_compare);
   print_map_line("duplicate4", map);
   PObjMapFind(&map, (void*)(intptr_t)2, int_compare);
   print_map_line("find2", map);
   PObjMapFind(&map, (void*)(intptr_t)1, int_compare);
   print_map_line("miss1", map);
   PObjMapFind(&map, (void*)(intptr_t)9, int_compare);
   print_map_line("miss9", map);
   PObjMapFind(&map, (void*)(intptr_t)4, int_compare);
   print_map_line("find4", map);
   PObjMapExtract(&map, (void*)(intptr_t)5, int_compare);
   print_map_line("extract_miss5", map);
   PObjMapExtract(&map, (void*)(intptr_t)3, int_compare);
   print_map_line("extract3", map);
   PObjMapExtract(&map, (void*)(intptr_t)2, int_compare);
   print_map_line("extract2", map);
   PObjMapFree(map);

   map = NULL;
   bool created = false;
   PObjMapGetRef(&map, (void*)(intptr_t)7, int_compare, &created);
   print_map_line(created ? "null_created" : "null_existing", map);
   PObjMapFind(&map, (void*)(intptr_t)7, int_compare);
   print_map_line("null_find", map);
   PObjMapExtract(&map, (void*)(intptr_t)7, int_compare);
   print_map_line("null_extract", map);

   map = NULL;
   PObjMapStore(&map, (void*)(intptr_t)3, (void*)(intptr_t)30, int_compare);
   PObjMapStore(&map, (void*)(intptr_t)1, (void*)(intptr_t)10, int_compare);
   PObjMapStore(&map, (void*)(intptr_t)2, (void*)(intptr_t)20, int_compare);
   printf("map free=");
   first_deleted = true;
   PObjMapFreeWDeleter(map, print_map_delete);
   putchar('\n');
   return 0;
}
