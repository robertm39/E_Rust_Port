#include <clb_stringtrees.h>

#include <stdbool.h>
#include <stdio.h>
#include <string.h>

static IntOrP int_value(long value)
{
   IntOrP result;
   result.i_val = value;
   return result;
}

static bool store(StrTree_p* tree, char* key, long val1, long val2)
{
   return StrTreeStore(tree, key, int_value(val1), int_value(val2)) != NULL;
}

static void print_key(const char* key)
{
   const unsigned char* cursor = (const unsigned char*)key;
   while(*cursor)
   {
      if(*cursor >= 0x20 && *cursor <= 0x7e && *cursor != '\\')
      {
         putchar(*cursor);
      }
      else
      {
         printf("\\x%02x", *cursor);
      }
      cursor++;
   }
}

static void print_tree(StrTree_p tree)
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

static void print_tree_line(const char* label, StrTree_p tree)
{
   printf("%s=", label);
   print_tree(tree);
   putchar('\n');
}

static void print_traversal(const char* label, PStack_p stack)
{
   bool first = true;
   printf("%s=", label);
   StrTree_p node;
   while((node = StrTreeTraverseNext(stack)))
   {
      if(!first)
      {
         putchar(',');
      }
      print_key(node->key);
      first = false;
   }
   putchar('\n');
   StrTreeTraverseExit(stack);
}

int main(void)
{
   setvbuf(stdout, NULL, _IONBF, 0);
   StrTree_p tree = NULL;
   print_tree_line("empty", tree);
   store(&tree, "d", 40, 400);
   print_tree_line("store_d", tree);
   store(&tree, "b", 20, 200);
   print_tree_line("store_b", tree);
   store(&tree, "f", 60, 600);
   print_tree_line("store_f", tree);
   store(&tree, "c", 30, 300);
   print_tree_line("store_c", tree);
   printf("duplicate_d=%d\n", store(&tree, "d", 44, 444));
   print_tree_line("duplicate_d_tree", tree);
   printf("values_d=%ld,%ld\n", tree->val1.i_val, tree->val2.i_val);
   printf("find_b=%d\n", StrTreeFind(&tree, "b") != NULL);
   print_tree_line("find_b_tree", tree);
   printf("miss_a=%d\n", StrTreeFind(&tree, "a") != NULL);
   print_tree_line("miss_a_tree", tree);
   printf("miss_z=%d\n", StrTreeFind(&tree, "z") != NULL);
   print_tree_line("miss_z_tree", tree);
   printf("find_d=%d\n", StrTreeFind(&tree, "d") != NULL);
   print_tree_line("find_d_tree", tree);
   StrTree_p removed = StrTreeExtractEntry(&tree, "e");
   printf("extract_miss_e=%d\n", removed != NULL);
   print_tree_line("extract_miss_e_tree", tree);
   removed = StrTreeExtractEntry(&tree, "c");
   printf("extract_c=%d\n", removed != NULL);
   print_tree_line("extract_c_tree", tree);
   StrTreeFree(removed);
   StrTreeFree(tree);

   tree = NULL;
   store(&tree, "e", 50, 0);
   store(&tree, "a", 10, 0);
   store(&tree, "c", 30, 0);
   store(&tree, "g", 70, 0);
   print_traversal("all", StrTreeTraverseInit(tree));
   StrTreeFree(tree);

   char embedded[] = {'a', 'l', 'p', 'h', 'a', '\0', 'x', '\0'};
   tree = NULL;
   printf("embedded_store=%d\n", store(&tree, embedded, 1, 10));
   printf("embedded_stored_length=%zu\n", strlen(tree->key));
   printf("embedded_duplicate_alpha=%d\n", store(&tree, "alpha", 99, 99));
   printf("embedded_find=%d\n", StrTreeFind(&tree, embedded) != NULL);
   removed = StrTreeExtractEntry(&tree, embedded);
   printf("embedded_extract_key=%s\n", removed->key);
   StrTreeFree(removed);

   char non_ascii[] = {(char)0xc3, (char)0xa9, 'c', 'l', 'a', 'i', 'r', '\0'};
   tree = NULL;
   store(&tree, "zeta", 5, 0);
   store(&tree, non_ascii, 4, 0);
   store(&tree, "alpha", 1, 0);
   print_traversal("byte_order", StrTreeTraverseInit(tree));
   StrTreeFree(tree);
   return 0;
}
