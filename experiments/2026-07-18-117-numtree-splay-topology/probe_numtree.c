#include <clb_numtrees.h>

#include <stdbool.h>
#include <stdio.h>

static IntOrP int_value(long value)
{
   IntOrP result;
   result.i_val = value;
   return result;
}

static bool store(NumTree_p* tree, long key, long val1, long val2)
{
   return NumTreeStore(tree, key, int_value(val1), int_value(val2));
}

static void print_tree(NumTree_p tree)
{
   if(!tree)
   {
      putchar('.');
      return;
   }
   printf("[%ld](", tree->key);
   print_tree(tree->lson);
   putchar(',');
   print_tree(tree->rson);
   putchar(')');
}

static void print_tree_line(const char* label, NumTree_p tree)
{
   printf("%s=", label);
   print_tree(tree);
   putchar('\n');
}

static void print_traversal(const char* label, PStack_p stack)
{
   bool first = true;
   printf("%s=", label);
   NumTree_p node;
   while((node = NumTreeTraverseNext(stack)))
   {
      printf("%s%ld", first ? "" : ",", node->key);
      first = false;
   }
   putchar('\n');
   NumTreeTraverseExit(stack);
}

int main(void)
{
   setvbuf(stdout, NULL, _IONBF, 0);
   NumTree_p tree = NULL;
   print_tree_line("empty", tree);
   store(&tree, 4, 40, 400);
   print_tree_line("store4", tree);
   store(&tree, 2, 20, 200);
   print_tree_line("store2", tree);
   store(&tree, 6, 60, 600);
   print_tree_line("store6", tree);
   store(&tree, 3, 30, 300);
   print_tree_line("store3", tree);
   store(&tree, 4, 44, 444);
   print_tree_line("duplicate4", tree);
   printf("values4=%ld,%ld\n", tree->val1.i_val, tree->val2.i_val);
   NumTreeFind(&tree, 2);
   print_tree_line("find2", tree);
   NumTreeMaxNode(tree);
   print_tree_line("max", tree);
   NumTreeFind(&tree, 1);
   print_tree_line("miss1", tree);
   NumTreeFind(&tree, 9);
   print_tree_line("miss9", tree);
   NumTreeFind(&tree, 4);
   print_tree_line("find4", tree);
   NumTree_p removed = NumTreeExtractEntry(&tree, 5);
   print_tree_line("extract_miss5", tree);
   if(removed)
   {
      NumTreeFree(removed);
   }
   removed = NumTreeExtractEntry(&tree, 3);
   print_tree_line("extract3", tree);
   NumTreeFree(removed);
   removed = NumTreeExtractRoot(&tree);
   print_tree_line("extract_root", tree);
   NumTreeFree(removed);
   NumTreeFree(tree);

   tree = NULL;
   store(&tree, 5, 50, 0);
   store(&tree, 1, 10, 0);
   store(&tree, 3, 30, 0);
   store(&tree, 7, 70, 0);
   printf("nodes=%ld\n", NumTreeNodes(tree));
   print_traversal("all", NumTreeTraverseInit(tree));
   print_traversal("limit4", NumTreeLimitedTraverseInit(tree, 4));
   print_traversal("limit5", NumTreeLimitedTraverseInit(tree, 5));
   print_traversal("limit8", NumTreeLimitedTraverseInit(tree, 8));
   puts("debug_keys_begin");
   NumTreeDebugPrint(stdout, tree, true);
   puts("debug_keys_end");
   NumTreeFree(tree);
   return 0;
}
