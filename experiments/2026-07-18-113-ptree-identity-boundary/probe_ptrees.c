#include "clb_ptrees.h"

#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>

static void print_node(PTree_p node)
{
   if(!node)
   {
      putchar('.');
      return;
   }
   printf("[%" PRIuPTR "](", (uintptr_t)node->key);
   print_node(node->lson);
   putchar(',');
   print_node(node->rson);
   putchar(')');
}

static void print_tree(const char* label, PTree_p root)
{
   printf("%s=", label);
   print_node(root);
   putchar('\n');
}

static void store(PTree_p* root, const char* label, uintptr_t key)
{
   printf("%s.inserted=%d\n", label, PTreeStore(root, (void*)key));
   print_tree(label, *root);
}

static void find(PTree_p* root, const char* label, uintptr_t key)
{
   PTree_p found = PTreeFind(root, (void*)key);
   printf("%s.found=%" PRIuPTR "\n", label,
          found ? (uintptr_t)found->key : 0);
   print_tree(label, *root);
}

int main(void)
{
   PTree_p root = NULL;
   PTree_p extracted;
   PTree_p binary;

   print_tree("initial", root);
   store(&root, "store-a", 4);
   store(&root, "store-b", 2);
   store(&root, "store-c", 6);
   store(&root, "store-d", 3);
   store(&root, "duplicate-a", 4);
   find(&root, "find-b", 2);
   binary = PTreeFindBinary(root, (void*)6);
   printf("binary-c.found=%" PRIuPTR "\n",
          binary ? (uintptr_t)binary->key : 0);
   print_tree("binary-c", root);
   find(&root, "miss-low", 1);
   find(&root, "miss-high", 9);
   find(&root, "find-a", 4);
   extracted = PTreeExtractEntry(&root, (void*)5);
   printf("extract-middle.found=%" PRIuPTR "\n",
          extracted ? (uintptr_t)extracted->key : 0);
   print_tree("extract-middle", root);
   extracted = PTreeExtractEntry(&root, (void*)3);
   printf("extract-d.found=%" PRIuPTR "\n",
          extracted ? (uintptr_t)extracted->key : 0);
   if(extracted)
   {
      PTreeCellFree(extracted);
   }
   print_tree("extract-d", root);
   printf("delete-c.deleted=%d\n", PTreeDeleteEntry(&root, (void*)6));
   print_tree("delete-c", root);
   PTreeFree(root);
   return 0;
}
