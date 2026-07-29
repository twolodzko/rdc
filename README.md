# `dc` implemented in Rust

The [desk calculator `dc`] re-implemented in Rust. It supports unlimited precision [fixed-point arithmetic].
Like `dc` it uses reverse-polish notation.

## Numbers

The [fixed-point numbers] are represented by integer part and fractional part, using dot as a deliminator,
for example `3.14159` or `42` are valid numbers. The exponential notation used commonly for floating-point
numbers is not supported. Negative numbers are prefixed by `_`, for example `_5`.

## Commands

The following subset of `ds`'s commands is implemented:

* `+`, `-`, `*`, `/`, `%` (reminder) arithmetic operations.
* `^` is exponentiation, where the base needs to be an integer.
* `v` square root.
* `d` duplicate the last value on the stack.
* `c` clear the stack.
* `r` reverse the order of the two last values on the stack.
* `p` pop the last value from the stack and print it followed by a newline character.
* `n` like above, but without the newline.
* `f` print whole stack (mostly for debugging).
* `k` pop the last value from the stack and use it to set the precision.
* `s`*r* pop the last value from the stack and save it to the record indexed by *r* (ASCII character).
* `l`*r* copy the value from the record *r* and push it to the top of the stack.
* `x` pop the last value from the stack and execute it as a command, if it is a number, push it to the stack.
* `>`*r*, `<`*r*, `=`*r*, `!>`*r*, `!<`*r*, `!=`*r* pop two values from the stack, compare them, and conditionally execute the value at the record *r* as a command.
* `[hello, world!]` is a string value equal to "hello, world!".
* `q` exit program.
* `j` pop the last value from the stack and use it to set the seed of the pseudo-random number generator.
* `'` generate a pseudo-random number using [LCG] and push it into the stack.

See also the [`dc` manual].

[desk calculator `dc`]: https://en.wikipedia.org/wiki/Dc_(computer_program)
[fixed-point arithmetic]: https://en.wikipedia.org/wiki/Fixed-point_arithmetic
[fixed-point numbers]: https://web.archive.org/web/20020611080806/http://www.embedded.com/98/9804fe2.htm
[`dc` manual]: https://www.gnu.org/software/bc/manual/dc-1.05/html_mono/dc.html
[LCG]: https://en.wikipedia.org/wiki/Linear_congruential_generator
