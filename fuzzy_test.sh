#!/bin/bash
set -e

BINARY="./target/debug/rdc"
PASSED=0
FAILED=0

echo "Test parsing numbers"
for _ in {0..500}; do
   s=$(($RANDOM % 10))
   val="$(bc <<< "scale = $s; $(($RANDOM % 10000)) / (10^$s)")"
   cmd="$val p"
   printf "Test:  %-25s  " "$cmd"

   result="$($BINARY "$cmd")"
   if [ $? -ne 0 ]; then
      echo "Error: command $BINARY '$cmd' failed"
      exit 1
   fi

   if [ "$result" != "$val" ]; then
      echo "FAIL"
      printf "#      got: %s\n" "$result"
      printf "# expected: %s\n" "$val"
      FAILED=$((FAILED+1))
   else
      PASSED=$((PASSED+1))
      echo "OK"
   fi
done
echo "====================================="

echo "Test sqrt"
for _ in {0..100}; do
   for prec in {0..5}; do
      s=$(($RANDOM % 10))
      val="$(bc <<< "scale = $s; $(($RANDOM % 10000)) / (10^$s)")"
      cmd="$prec k $val vp"
      printf "Test:  %-25s  " "$cmd"

      result="$($BINARY "$cmd")"
      if [ $? -ne 0 ]; then
         echo "Error: command $BINARY '$cmd' failed"
         exit 1
      fi

      expected="$(dc -e "$cmd")"
      if [ $? -ne 0 ]; then
         echo "Error: command dc -e '$cmd' failed"
         exit 1
      fi

      if [ "$result" != "$expected" ]; then
         echo "FAIL"
         printf "#      got: %s\n" "$result"
         printf "# expected: %s\n" "$expected"
         FAILED=$((FAILED+1))
      else
         PASSED=$((PASSED+1))
         echo "OK"
      fi
   done
done
echo "====================================="

echo "Test bivariate operations"
for _ in {1..10}; do
   for prec in {0..5}; do
      for dx in {0..5}; do
         for dy in {0..5}; do
            for op in '+' '-' '*' '/' '%' '^'; do
               if [ "$op" = '^' ]; then
                  # dc would throw an error for non-integer exponent
                  dy=0
               fi

               x="$(bc <<< "scale = $dx; $(($RANDOM % 1000)) / ($dx+1)")"
               y="$(bc <<< "scale = $dy; $(($RANDOM % 1000)) / ($dy+1)")"

               # ignore division by zero error handling
               if [[ "$op" = "^" || "$op" = "/" || "$op" = "%" ]] && [[ "$y" = "0" ]]; then
                  continue
               fi

               cmd="$prec k $x $y $op p"
               printf "Test:  %-25s  " "$cmd"

               result="$($BINARY "$cmd")"
               if [ $? -ne 0 ]; then
                  echo "Error: command $BINARY '$cmd' failed"
                  exit 1
               fi

               expected="$(dc -e "$cmd" | tr -d '\\ \n' )"
               if [ $? -ne 0 ]; then
                  echo "Error: command dc -e '$cmd' failed"
                  exit 1
               fi

               if [ "$op" = "^" ] || [ "$op" = "%" ]; then
                  # TODO: this can be improved later
                  # remove tail zeros in the fractional part
                  result="$(sed -E 's/^\.0+$/0/; s/\.0+$//; /\.[0-9]/ s/0+$//' <<< "$result")"
                  expected="$(sed -E 's/^\.0+$/0/; s/\.0+$//; /\.[0-9]/ s/0+$//' <<< "$expected")"
               fi

               if [ "$result" != "$expected" ]; then
                  echo "FAIL"
                  printf "#      got: %s\n" "$result"
                  printf "# expected: %s\n" "$expected"
                  FAILED=$((FAILED+1))
               else
                  PASSED=$((PASSED+1))
                  echo "OK"
               fi
            done
         done
      done
   done
done
echo "====================================="

echo "Test comparison operations"
for _ in {1..10}; do
   for dx in {0..5}; do
      for dy in {0..5}; do
         x="$(bc <<< "scale = $dx; $(($RANDOM % 1000)) / ($dx+1)")"
         y="$(bc <<< "scale = $dy; $(($RANDOM % 1000)) / ($dy+1)")"

         for op in '=' '<' '>' '!=' '!<' '!>'; do

            cmd="[[yes]pq]sa $x $y ${op}a [no]p"
            printf "Test:  %-25s  " "$cmd"

            result="$($BINARY "$cmd")"
            if [ $? -ne 0 ]; then
               echo "Error: command $BINARY '$cmd' failed"
               exit 1
            fi

            expected="$(dc -e "$cmd" | tr -d '\\ \n' )"
            if [ $? -ne 0 ]; then
               echo "Error: command dc -e '$cmd' failed"
               exit 1
            fi

            if [ "$result" != "$expected" ]; then
               echo "FAIL"
               printf "#      got: %s\n" "$result"
               printf "# expected: %s\n" "$expected"
               FAILED=$((FAILED+1))
            else
               PASSED=$((PASSED+1))
               echo "OK"
            fi
         done
      done
   done
done
echo "====================================="

TOTAL=$((FAILED + PASSED))
echo "Passed: $PASSED  $((PASSED * 100 / TOTAL))%"
echo "Failed: $FAILED  $((FAILED * 100 / TOTAL))%"
