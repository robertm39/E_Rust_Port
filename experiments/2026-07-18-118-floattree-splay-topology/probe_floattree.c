#include <clb_floattrees.h>

#include <inttypes.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static IntOrP int_value(long value)
{
   IntOrP result;
   result.i_val = value;
   return result;
}

static bool store(FloatTree_p* tree, double key, long val1, long val2)
{
   return FloatTreeStore(tree, key, int_value(val1), int_value(val2));
}

static uint64_t key_bits(double key)
{
   uint64_t result;
   memcpy(&result, &key, sizeof(result));
   return result;
}

static double key_from_bits(uint64_t bits)
{
   double result;
   memcpy(&result, &bits, sizeof(result));
   return result;
}

static void print_key(double key)
{
   if(isnan(key))
   {
      printf("nan@%016" PRIx64, key_bits(key));
   }
   else if(key == 0.0)
   {
      printf("%c0", signbit(key) ? '-' : '+');
   }
   else
   {
      printf("%.17g", key);
   }
}

static void print_tree(FloatTree_p tree)
{
   if(!tree)
   {
      putchar('.');
      return;
   }
   putchar('[');
   print_key(tree->key);
   putchar(']');
   putchar('(');
   print_tree(tree->lson);
   putchar(',');
   print_tree(tree->rson);
   putchar(')');
}

static void print_tree_line(const char* label, FloatTree_p tree)
{
   printf("%s=", label);
   print_tree(tree);
   putchar('\n');
}

static void print_traversal(const char* label, PStack_p stack)
{
   bool first = true;
   printf("%s=", label);
   FloatTree_p node;
   while((node = FloatTreeTraverseNext(stack)))
   {
      if(!first)
      {
         putchar(',');
      }
      print_key(node->key);
      first = false;
   }
   putchar('\n');
   FloatTreeTraverseExit(stack);
}

int main(void)
{
   setvbuf(stdout, NULL, _IONBF, 0);
   FloatTree_p tree = NULL;
   print_tree_line("empty", tree);
   store(&tree, 4.0, 40, 400);
   print_tree_line("store4", tree);
   store(&tree, 2.0, 20, 200);
   print_tree_line("store2", tree);
   store(&tree, 6.0, 60, 600);
   print_tree_line("store6", tree);
   store(&tree, 3.0, 30, 300);
   print_tree_line("store3", tree);
   printf("duplicate4=%d\n", store(&tree, 4.0, 44, 444));
   print_tree_line("duplicate4_tree", tree);
   printf("values4=%ld,%ld\n", tree->val1.i_val, tree->val2.i_val);
   printf("find2=%d\n", FloatTreeFind(&tree, 2.0) != NULL);
   print_tree_line("find2_tree", tree);
   printf("miss1=%d\n", FloatTreeFind(&tree, 1.0) != NULL);
   print_tree_line("miss1_tree", tree);
   printf("miss9=%d\n", FloatTreeFind(&tree, 9.0) != NULL);
   print_tree_line("miss9_tree", tree);
   printf("find4=%d\n", FloatTreeFind(&tree, 4.0) != NULL);
   print_tree_line("find4_tree", tree);
   FloatTree_p removed = FloatTreeExtractEntry(&tree, 5.0);
   printf("extract_miss5=%d\n", removed != NULL);
   print_tree_line("extract_miss5_tree", tree);
   removed = FloatTreeExtractEntry(&tree, 3.0);
   printf("extract3=%d\n", removed != NULL);
   print_tree_line("extract3_tree", tree);
   FloatTreeFree(removed);
   FloatTreeFree(tree);

   tree = NULL;
   store(&tree, 5.0, 50, 0);
   store(&tree, 1.0, 10, 0);
   store(&tree, 3.0, 30, 0);
   store(&tree, 7.0, 70, 0);
   printf("nodes=%ld\n", FloatTreeNodes(tree));
   print_traversal("all", FloatTreeTraverseInit(tree));
   FloatTreeFree(tree);

   tree = NULL;
   printf("store_neg_zero=%d\n", store(&tree, -0.0, 10, 100));
   printf("store_pos_zero=%d\n", store(&tree, 0.0, 99, 999));
   printf("zero_root_bits=%016" PRIx64 "\n", key_bits(tree->key));
   printf("find_pos_zero=%d\n", FloatTreeFind(&tree, 0.0) != NULL);
   removed = FloatTreeExtractEntry(&tree, 0.0);
   printf("extract_pos_zero_bits=%016" PRIx64 "\n", key_bits(removed->key));
   FloatTreeFree(removed);

   double nan1 = key_from_bits(UINT64_C(0x7ff8000000000001));
   double nan2 = key_from_bits(UINT64_C(0x7ff8000000000002));
   tree = NULL;
   store(&tree, 1.0, 10, 100);
   store(&tree, 2.0, 20, 200);
   printf("numeric_store_nan=%d\n", store(&tree, nan1, 99, 999));
   print_tree_line("numeric_after_store_nan", tree);
   printf("numeric_root_values=%ld,%ld\n", tree->val1.i_val, tree->val2.i_val);
   printf("numeric_find_nan=%d\n", FloatTreeFind(&tree, nan1) != NULL);
   print_tree_line("numeric_after_find_nan", tree);
   printf("numeric_extract_nan=%d\n", FloatTreeExtractEntry(&tree, nan1) != NULL);
   print_tree_line("numeric_after_extract_nan", tree);
   FloatTreeFree(tree);

   tree = NULL;
   printf("empty_store_nan=%d\n", store(&tree, nan1, 20, 200));
   print_tree_line("nan_root", tree);
   printf("nan_find_self=%d\n", FloatTreeFind(&tree, nan1) != NULL);
   printf("nan_find_one=%d\n", FloatTreeFind(&tree, 1.0) != NULL);
   printf("nan_store_one=%d\n", store(&tree, 1.0, 10, 100));
   printf("nan_store_other_nan=%d\n", store(&tree, nan2, 99, 999));
   printf("nan_extract_self=%d\n", FloatTreeExtractEntry(&tree, nan1) != NULL);
   printf("nan_delete_self=%d\n", FloatTreeDeleteEntry(&tree, nan1));
   printf("nan_nodes=%ld\n", FloatTreeNodes(tree));
   print_traversal("nan_all", FloatTreeTraverseInit(tree));
   FloatTreeFree(tree);
   return 0;
}
