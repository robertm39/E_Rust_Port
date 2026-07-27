#include "clb_quadtrees.h"

#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>

static QuadKey make_key(uintptr_t p1, int i1, uintptr_t p2, int i2)
{
   QuadKey key;
   key.p1 = (void*)p1;
   key.i1 = i1;
   key.p2 = (void*)p2;
   key.i2 = i2;
   return key;
}

static void print_node(QuadTree_p node)
{
   if(!node)
   {
      putchar('.');
      return;
   }
   printf("[%" PRIuPTR ",%d,%" PRIuPTR ",%d]=%ld(",
          (uintptr_t)node->key.p1,
          node->key.i1,
          (uintptr_t)node->key.p2,
          node->key.i2,
          node->val.i_val);
   print_node(node->lson);
   putchar(',');
   print_node(node->rson);
   putchar(')');
}

static void print_tree(const char* label, QuadTree_p root)
{
   printf("%s=", label);
   print_node(root);
   putchar('\n');
}

static void store(QuadTree_p* root, const char* label, QuadKey* key, long value)
{
   IntOrP val;
   val.i_val = value;
   printf("%s.inserted=%d\n", label, QuadTreeStore(root, key, val));
   print_tree(label, *root);
}

static void find(QuadTree_p* root, const char* label, QuadKey* key)
{
   QuadTree_p found = QuadTreeFind(root, key);
   printf("%s.found=%ld\n", label, found ? found->val.i_val : -1L);
   print_tree(label, *root);
}

int main(void)
{
   QuadTree_p root = NULL;
   QuadTree_p extracted;
   QuadKey a = make_key(4, 0, 40, 0);
   QuadKey b = make_key(2, 0, 20, 0);
   QuadKey c = make_key(6, 0, 60, 0);
   QuadKey d = make_key(4, -1, 40, 0);
   QuadKey low = make_key(1, 0, 10, 0);
   QuadKey high = make_key(9, 0, 90, 0);
   QuadKey middle_miss = make_key(5, 0, 50, 0);

   print_tree("initial", root);
   store(&root, "store-a", &a, 40);
   store(&root, "store-b", &b, 20);
   store(&root, "store-c", &c, 60);
   store(&root, "store-d", &d, 39);
   store(&root, "duplicate-a", &a, 999);
   find(&root, "find-b", &b);
   find(&root, "miss-low", &low);
   find(&root, "miss-high", &high);
   find(&root, "find-a", &a);
   extracted = QuadTreeExtractEntry(&root, &middle_miss);
   printf("extract-middle.found=%ld\n", extracted ? extracted->val.i_val : -1L);
   print_tree("extract-middle", root);
   extracted = QuadTreeExtractEntry(&root, &d);
   printf("extract-d.found=%ld\n", extracted ? extracted->val.i_val : -1L);
   if(extracted)
   {
      QuadTreeCellFree(extracted);
   }
   print_tree("extract-d", root);
   printf("delete-c.deleted=%d\n", QuadTreeDeleteEntry(&root, &c));
   print_tree("delete-c", root);
   QuadTreeFree(root);
   return 0;
}
