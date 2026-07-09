#!/usr/bin/env bats
# shellcheck disable=SC2016

BINARY="./target/debug/rdc"

@test "Greeting" {
	run $BINARY '[hello, world!]p'
	[ "$output" = "hello, world!" ]
	[ "$status" -eq 0 ]
}

@test "Push and print 123456789" {
	run $BINARY '123456789 p'
	[ "$output" = "123456789" ]
	[ "$status" -eq 0 ]
}

@test "Push and print 123456789.0" {
	run $BINARY '123456789.0 p'
	[ "$output" = "123456789.0" ]
	[ "$status" -eq 0 ]
}

@test "Push and print 123.456789" {
	run $BINARY '123.456789 p'
	[ "$output" = "123.456789" ]
	[ "$status" -eq 0 ]
}

@test "Push and print 123456.789" {
	run $BINARY '123456.789 p'
	[ "$output" = "123456.789" ]
	[ "$status" -eq 0 ]
}

@test "Push and print 0.123456789" {
	run $BINARY '.123456789 p'
	[ "$output" = ".123456789" ]
	[ "$status" -eq 0 ]
}

@test "Push and print 0.000123456" {
	run $BINARY '.000123456 p'
	[ "$output" = ".000123456" ]
	[ "$status" -eq 0 ]
}

@test "Push and print -123456789" {
	run $BINARY '_123456789 p'
	[ "$output" = "-123456789" ]
	[ "$status" -eq 0 ]
}

@test "2 + 2 = 4" {
	run $BINARY '2 2 + p'
	[ "$output" = "4" ]
	[ "$status" -eq 0 ]
}

@test "3 * 2 = 6" {
	run $BINARY '3 2 * p'
	[ "$output" = "6" ]
	[ "$status" -eq 0 ]
}

@test "14 / 2 = 7" {
	run $BINARY '14 2 / p'
	[ "$output" = "7" ]
	[ "$status" -eq 0 ]
}

@test "-2^2 = 4" {
	run $BINARY '_2 2 ^ p'
	[ "$output" = "4" ]
	[ "$status" -eq 0 ]
}

@test "Bug: precision overflow: 0k 38 28.0 /p" {
	run $BINARY '0k 38 28.0 /p'
	[ "$output" = "1" ]
	[ "$status" -eq 0 ]
}

@test "Bug: precision in div: 0k 4492.5 639.0 /p" {
	run $BINARY '0k 4492.5 639.0 /p'
	[ "$output" = "7" ]
	[ "$status" -eq 0 ]
}

@test "Bug: precision in rem: 0k 2456.5 1447.0 %p" {
	run $BINARY '0k 2456.5 1447.0 %p'
	[ "$output" = "1009.5" ]
	[ "$status" -eq 0 ]
}

@test "Square value using duplication" {
	run $BINARY '2d*p'
	[ "$output" = "4" ]
	[ "$status" -eq 0 ]
}

@test "Square root of 25" {
	run $BINARY '25vp'
	[ "$output" = "5" ]
	[ "$status" -eq 0 ]
}

@test "Print stack" {
	run $BINARY '1 2 3 f'
   [ "$output" = "1 2 3" ]
	[ "$status" -eq 0 ]
}

@test "Clean stack" {
	run $BINARY '1 2 3 cf'
   [ "$output" = "" ]
	[ "$status" -eq 0 ]
}


@test "Reverse stack" {
	run $BINARY '1 2 3 rf'
   [ "$output" = "1 3 2" ]
	[ "$status" -eq 0 ]
}

@test "Execute string" {
	run $BINARY '[40 2 +p]x'
   [ "$output" = "42" ]
	[ "$status" -eq 0 ]
}

@test "Use register" {
	run $BINARY '2 sa 4 la /p'
   [ "$output" = "2" ]
	[ "$status" -eq 0 ]
}

@test "Empty register is zero" {
	run $BINARY 'lxp'
   [ "$output" = "0" ]
	[ "$status" -eq 0 ]
}

@test "Calculate factorial" {
	run $BINARY '5 [d1-d1<F*]dsFxp'
   [ "$output" = "120" ]
	[ "$status" -eq 0 ]
}

@test "Execute and continue" {
	run $BINARY '[[everything]n]sa [[ is]n]sb 0 0=a 1 2!=b [ fine]p'
   [ "$output" = "everything is fine" ]
	[ "$status" -eq 0 ]
}

@test "Conditional call on 2 > 1" {
	run $BINARY '[[ok]p]sa 1 2>a'
   [ "$output" = "ok" ]
	[ "$status" -eq 0 ]
}

@test "Conditional call on 1 != 2" {
	run $BINARY '[[ok]p]sa 1 2!=a'
   [ "$output" = "ok" ]
	[ "$status" -eq 0 ]
}

@test "Conditional call on 8/2 = 3+1" {
	run $BINARY '[[also ok]p]sa 8 2 / 3 1 + =a'
   [ "$output" = "also ok" ]
	[ "$status" -eq 0 ]
}

@test "Infinite loop does not lead to stack overflow" {
	# run `yes` implemented in dc
	run timeout 1 $BINARY '[[yes]plyx]sylyx'
	# finished with the timeout error code
	[ "$status" -eq 124 ]
}

@test "Error on printing empty stack" {
	run $BINARY 'p'
   [ "$output" = "Error: empty stack" ]
	[ "$status" -ne 0 ]
}

@test "Error on adding NaNs" {
	run $BINARY '[] [] +'
   [ "$output" = "Error: not a number" ]
	[ "$status" -ne 0 ]
}
