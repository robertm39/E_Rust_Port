#include <clb_numxtrees.h>

#include <stdbool.h>
#include <stdio.h>

static IntOrP int_value(long value)
{
   IntOrP result;
   result.i_val = value;
   return result;
}

static bool store(NumXTree_p* tree, long key, long val1, long val2)
{
   return NumXTreeStore(tree, key, int_value(val1), int_value(val2));
}

static void print_tree(NumXTree_p tree)
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

static void print_tree_line(const char* label, NumXTree_p tree)
{
   printf("%s=", label);
   print_tree(tree);
   putchar('\n');
}

static void print_traversal(const char* label, PStack_p stack)
{
   bool first = true;
   printf("%s=", label);
   NumXTree_p node;
   while((node = NumXTreeTraverseNext(stack)))
   {
      printf("%s%ld", first ? "" : ",", node->key);
      first = false;
   }
   putchar('\n');
   NumXTreeTraverseExit(stack);
}

int main(void)
{
   setvbuf(stdout, NULL, _IONBF, 0);
   NumXTree_p tree = NULL;
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
   printf("values4=%ld,%ld\n", tree->vals[0].i_val, tree->vals[1].i_val);
   NumXTreeFind(&tree, 2);
   print_tree_line("find2", tree);
   NumXTreeMaxNode(tree);
   print_tree_line("max", tree);
   NumXTreeFind(&tree, 1);
   print_tree_line("miss1", tree);
   NumXTreeFind(&tree, 9);
   print_tree_line("miss9", tree);
   NumXTreeFind(&tree, 4);
   print_tree_line("find4", tree);
   NumXTree_p removed = NumXTreeExtractEntry(&tree, 5);
   print_tree_line("extract_miss5", tree);
   if(removed)
   {
      NumXTreeFree(removed);
   }
   removed = NumXTreeExtractEntry(&tree, 3);
   print_tree_line("extract3", tree);
   NumXTreeFree(removed);
   removed = NumXTreeExtractRoot(&tree);
   print_tree_line("extract_root", tree);
   NumXTreeFree(removed);
   NumXTreeFree(tree);

   tree = NULL;
   store(&tree, 5, 50, 0);
   store(&tree, 1, 10, 0);
   store(&tree, 3, 30, 0);
   store(&tree, 7, 70, 0);
   printf("nodes=%ld\n", NumXTreeNodes(tree));
   print_traversal("all", NumXTreeTraverseInit(tree));
   print_traversal("limit4", NumXTreeLimitedTraverseInit(tree, 4));
   print_traversal("limit5", NumXTreeLimitedTraverseInit(tree, 5));
   print_traversal("limit8", NumXTreeLimitedTraverseInit(tree, 8));
   NumXTreeFree(tree);
   return 0;
}
