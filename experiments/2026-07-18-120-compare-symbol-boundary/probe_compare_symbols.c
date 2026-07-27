#include <stdio.h>

#include "clb_partial_orderings.h"

int main(void)
{
   printf("enum=%d,%d,%d,%d,%d,%d,%d\n",
          to_unknown,
          to_uncomparable,
          to_equal,
          to_greater,
          to_lesser,
          to_notgteq,
          to_notleeq);
   printf("symbols=0:%s|1:%s|2:%s|3:%s|4:%s\n",
          POCompareSymbol[to_unknown],
          POCompareSymbol[to_uncomparable],
          POCompareSymbol[to_equal],
          POCompareSymbol[to_greater],
          POCompareSymbol[to_lesser]);
   printf("inverse=1:%d|2:%d|3:%d|4:%d|5:%d|6:%d\n",
          POInverseRelation(to_uncomparable),
          POInverseRelation(to_equal),
          POInverseRelation(to_greater),
          POInverseRelation(to_lesser),
          POInverseRelation(to_notgteq),
          POInverseRelation(to_notleeq));
   printf("q_to_part=-7:%d|0:%d|9:%d\n",
          Q_TO_PART(-7),
          Q_TO_PART(0),
          Q_TO_PART(9));
   return 0;
}
