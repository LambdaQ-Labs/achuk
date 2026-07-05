# The Achuk language in 10 minutes

Achuk's surface syntax is a small, ML-family functional language (it inherits
from Roc). Everything below is runnable today with `achuk run`.

## Values and functions

```achuk
name = "Ada"                 # a binding
double = |n| n * 2           # a function (lambda)
add = |a, b| a + b           # two parameters
```

Functions are called with parentheses, or with method-style dot syntax:

```achuk
add(2, 3)        # 5
3.to_str()       # "3" — method call form
```

## Strings

```achuk
greeting = "Hello, ${name}!"     # interpolation with ${...}
Str.join_with(["a", "b"], ", ")  # "a, b"
```

## Numbers

Numeric literals default to a decimal type, so `21 * 2` prints `42.0`. Use
`Num`/`Nat`/`Int` operations for arithmetic and `.to_str()` to render.

```achuk
x = 21 * 2
x.to_str()       # "42.0"
7 % 3            # modulo -> 1
```

## Conditionals

```achuk
sign = |n|
    if n > 0 "positive"
    else if n == 0 "zero"
    else "negative"
```

Booleans combine with `and` / `or` / `not`.

## Pattern matching

```achuk
classify = |n| match n {
    0 => "zero"
    1 => "one"
    _ => "many"
}
```

## Lists

```achuk
nums = [1, 2, 3]
nums.map(|n| n * 2)          # [2, 4, 6]
nums.len()                   # 3
```

## Blocks

Curly braces group a sequence of bindings ending in a result expression:

```achuk
area = |w, h| {
    a = w * h
    a
}
```

## The entry point

Every runnable program defines `main!`:

```achuk
main! = |args| {
    echo!("running with ${args.len().to_str()} args")
    Ok({})
}
```

- The `!` suffix marks an effectful function (one that can print, etc.).
- `args` is a `List Str` of command-line arguments.
- Return `Ok({})` for success, or `Err(...)` to exit with a failure.

## Errors and results

Fallible operations return a `Result` (`Ok`/`Err`), which you match on:

```achuk
main! = |_args| match risky() {
    Ok(v) => {
        echo!(v)
        Ok({})
    }
    Err(_) => Err(Exit(1))
}
```

## Common gotchas

A few sharp edges worth knowing early (all hit while writing the examples):

- **Numbers default to a decimal type.** `21 * 2` prints `42.0`, and `%`
  forces the decimal type. For clean integer output, annotate with `U64`
  (or `I64`): `fizz : U64 -> Str`. That's why the numeric examples carry
  type signatures.
- **Type applications need parentheses:** write `List(U64)`, not `List U64`,
  in a type annotation.
- **A bare `[]` right after a one-line `if` mis-parses** (it looks like
  indexing). Wrap it: `if n <= 0 ([]) else ...`.
- **Multi-line `if / else if / else` wants a block.** Put it inside `{ }`
  when it's a function body:
  ```achuk
  is_prime = |n| {
      if n < 2 False
      else if has_divisor(n, 2) False
      else True
  }
  ```
- **No prefix `not` keyword.** Restructure with `if/else`, or use the
  boolean the other way around.

## Where to go next

- [Getting started](getting-started.md) — install and project setup.
- [`examples/`](../examples) — hello, fizzbuzz, pattern matching, args, plus
  three fuller programs: `stats` (descriptive statistics), `primes` (trial
  division), and `gradebook` (averages + letter grades).
- Let an agent write Achuk for you: `achuk mcp install` (see getting-started).
