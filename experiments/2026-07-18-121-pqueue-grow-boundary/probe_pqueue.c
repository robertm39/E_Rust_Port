#include <stdio.h>

#include "clb_pqueue.h"

static void queue_init(PQueueCell* queue, long head, long tail,
                       const long values[4])
{
   long i;

   queue->size = 4;
   queue->head = head;
   queue->tail = tail;
   queue->queue = SizeMalloc(4*sizeof(IntOrP));
   for(i=0; i<4; i++)
   {
      queue->queue[i].i_val = values[i];
   }
}

static void queue_free(PQueueCell* queue)
{
   SizeFree(queue->queue, queue->size*sizeof(IntOrP));
}

static void print_drain(PQueueCell* queue)
{
   int first = 1;

   while(!PQueueEmpty(queue))
   {
      if(!first)
      {
         putchar(',');
      }
      printf("%ld", PQueueGetNextInt(queue));
      first = 0;
   }
   putchar('\n');
}

static void print_indices(PQueueCell* queue)
{
   long index;
   int first = 1;

   for(index=PQueueTailIndex(queue); index!=-1;
       index=PQueueIncIndex(queue, index))
   {
      if(!first)
      {
         putchar(',');
      }
      printf("%ld", index);
      first = 0;
   }
   putchar('\n');
}

int main(void)
{
   const long empty_values[4] = {-1, -1, -1, -1};
   const long direct_full_values[4] = {4, 1, 2, 3};
   const long direct_nonfull_values[4] = {10, 20, 30, 40};
   PQueueCell queue;

   queue_init(&queue, 0, 0, empty_values);
   PQueueStoreInt(&queue, 10);
   PQueueStoreInt(&queue, 20);
   PQueueStoreInt(&queue, 30);
   PQueueStoreInt(&queue, 40);
   printf("store=size:%ld,head:%ld,tail:%ld,card:%ld,slots:%ld,%ld,%ld,%ld,drain:",
          queue.size, queue.head, queue.tail, PQueueCardinality(&queue),
          PQueueElementInt(&queue, 4), PQueueElementInt(&queue, 5),
          PQueueElementInt(&queue, 6), PQueueElementInt(&queue, 7));
   print_drain(&queue);
   queue_free(&queue);

   queue_init(&queue, 0, 0, empty_values);
   PQueueBuryInt(&queue, 10);
   PQueueBuryInt(&queue, 20);
   PQueueBuryInt(&queue, 30);
   PQueueBuryInt(&queue, 40);
   printf("bury=size:%ld,head:%ld,tail:%ld,card:%ld,slots:%ld,%ld,%ld,%ld,drain:",
          queue.size, queue.head, queue.tail, PQueueCardinality(&queue),
          PQueueElementInt(&queue, 4), PQueueElementInt(&queue, 5),
          PQueueElementInt(&queue, 6), PQueueElementInt(&queue, 7));
   print_drain(&queue);
   queue_free(&queue);

   queue_init(&queue, 0, 0, empty_values);
   PQueueStoreInt(&queue, 1);
   PQueueStoreInt(&queue, 2);
   PQueueStoreInt(&queue, 3);
   (void)PQueueGetNextInt(&queue);
   PQueueStoreInt(&queue, 4);
   PQueueStoreInt(&queue, 5);
   printf("wrapped=size:%ld,head:%ld,tail:%ld,card:%ld,copied:%ld,%ld,%ld,%ld,drain:",
          queue.size, queue.head, queue.tail, PQueueCardinality(&queue),
          PQueueElementInt(&queue, 0), PQueueElementInt(&queue, 5),
          PQueueElementInt(&queue, 6), PQueueElementInt(&queue, 7));
   print_drain(&queue);
   queue_free(&queue);

   queue_init(&queue, 1, 1, direct_full_values);
   PQueueGrow(&queue);
   printf("direct_full=size:%ld,head:%ld,tail:%ld,card:%ld,copied:%ld,%ld,%ld,%ld,drain:",
          queue.size, queue.head, queue.tail, PQueueCardinality(&queue),
          PQueueElementInt(&queue, 0), PQueueElementInt(&queue, 5),
          PQueueElementInt(&queue, 6), PQueueElementInt(&queue, 7));
   print_drain(&queue);
   queue_free(&queue);

   queue_init(&queue, 2, 0, direct_nonfull_values);
   PQueueGrow(&queue);
   printf("direct_nonfull=size:%ld,head:%ld,tail:%ld,card:%ld,copied:%ld,%ld,%ld,%ld,indices:",
          queue.size, queue.head, queue.tail, PQueueCardinality(&queue),
          PQueueElementInt(&queue, 0), PQueueElementInt(&queue, 1),
          PQueueElementInt(&queue, 6), PQueueElementInt(&queue, 7));
   print_indices(&queue);
   queue_free(&queue);

   return 0;
}
