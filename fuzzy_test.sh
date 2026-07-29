#!/bin/bash
set -e

BINARY="./target/debug/rdc"
PASSED=0
FAILED=0
MAX_RAND=1000000
DC_ELAPSED=0
RDC_ELAPSED=0

echo "Test parsing numbers"
for _ in {0..500}; do
   s=$((RANDOM % 10))
   val="$(bc <<< "scale = $s; $((RANDOM % MAX_RAND)) / (10^$s)")"
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
      s=$((RANDOM % 10))
      val="$(bc <<< "scale = $s; $((RANDOM % MAX_RAND)) / (10^$s)")"
      cmd="$prec k $val vp"
      printf "Test:  %-25s  " "$cmd"

      start=$(date +%s.%N)
      result="$($BINARY "$cmd")"
      end=$(date +%s.%N)
      RDC_ELAPSED=$(echo "$RDC_ELAPSED + $end - $start" | bc -l)
      if [ $? -ne 0 ]; then
         echo "Error: command $BINARY '$cmd' failed"
         exit 1
      fi

      start=$(date +%s.%N)
      expected="$(dc -e "$cmd")"
      end=$(date +%s.%N)
      DC_ELAPSED=$(echo "$DC_ELAPSED + $end - $start" | bc -l)
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
for _ in {1..500}; do
   for op in '+' '-' '*' '/' '%' '^'; do
      prec=$((RANDOM % 20))
      dx=$((RANDOM % 5))
      x="$(bc <<< "scale = $dx; $((RANDOM % MAX_RAND)) / (10^$dx)")"
      dy=$((RANDOM % 5))
      if [ "$op" = '^' ]; then
         # dc would throw an error for non-integer exponent
         dy=0
      fi
      y="$(bc <<< "scale = $dy; $((RANDOM % MAX_RAND)) / (10^$dy)")"

      # ignore division by zero error handling
      if [[ "$op" = "^" || "$op" = "/" || "$op" = "%" ]] && [[ "$y" = "0" ]]; then
         continue
      fi

      cmd="$prec k $x $y $op p"
      printf "Test:  %-25s  " "$cmd"

      start=$(date +%s.%N)
      result="$($BINARY "$cmd")"
      end=$(date +%s.%N)
      RDC_ELAPSED=$(echo "$RDC_ELAPSED + $end - $start" | bc -l)
      if [ $? -ne 0 ]; then
         echo "Error: command $BINARY '$cmd' failed"
         exit 1
      fi

      start=$(date +%s.%N)
      expected="$(dc -e "$cmd" | tr -d '\\ \n' )"
      end=$(date +%s.%N)
      DC_ELAPSED=$(echo "$DC_ELAPSED + $end - $start" | bc -l)
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
echo "====================================="

echo "Test comparison operations"
for _ in {1..500}; do
   dx=$((RANDOM % 5))
   x="$(bc <<< "scale = $dx; $((RANDOM % MAX_RAND)) / (10^$dx)")"
   dy=$((RANDOM % 5))
   y="$(bc <<< "scale = $dy; $((RANDOM % MAX_RAND)) / (10^$dy)")"

   for op in '=' '<' '>' '!=' '!<' '!>'; do

      cmd="[[yes]pq]sa $x $y ${op}a [no]p"
      printf "Test:  %-25s  " "$cmd"

      start=$(date +%s.%N)
      result="$($BINARY "$cmd")"
      end=$(date +%s.%N)
      RDC_ELAPSED=$(echo "$RDC_ELAPSED + $end - $start" | bc -l)
      if [ $? -ne 0 ]; then
         echo "Error: command $BINARY '$cmd' failed"
         exit 1
      fi

      start=$(date +%s.%N)
      expected="$(dc -e "$cmd" | tr -d '\\ \n' )"
      end=$(date +%s.%N)
      DC_ELAPSED=$(echo "$DC_ELAPSED + $end - $start" | bc -l)
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

TOTAL=$((FAILED + PASSED))
echo "Passed: $PASSED  $((PASSED * 100 / TOTAL))%"
echo "Failed: $FAILED  $((FAILED * 100 / TOTAL))%"
echo "====================================="

echo "Runtimes:"
echo "dc:  $DC_ELAPSED sec"
echo "rdc: $RDC_ELAPSED sec"
echo "rdc/dc: $(echo "scale=3; $RDC_ELAPSED / $DC_ELAPSED" | bc -l)"
