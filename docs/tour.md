# The Claw language in 10 minutes

Claw's surface syntax is a small, ML-family functional language (it inherits
from Roc). Everything below is runnable today with `claw run`.

## Values and functions

```claw
name = "Ada"                 # a binding
double = |n| n * 2           # a function (lambda)
add = |a, b| a + b           # two parameters
```

Functions are called with parentheses, or with method-style dot syntax:

```claw
add(2, 3)        # 5
3.to_str()       # "3" — method call form
```

## Strings

```claw
greeting = "Hello, ${name}!"     # interpolation with ${...}
Str.join_with(["a", "b"], ", ")  # "a, b"
```

## Numbers

Numeric literals default to a decimal type, so `21 * 2` prints `42.0`. Use
`Num`/`Nat`/`Int` operations for arithmetic and `.to_str()` to render.

```claw
x = 21 * 2
x.to_str()       # "42.0"
7 % 3            # modulo -> 1
```

## Conditionals

```claw
sign = |n|
    if n > 0 "positive"
    else if n == 0 "zero"
    else "negative"
```

Booleans combine with `and` / `or` / `not`.

## Pattern matching

```claw
classify = |n| match n {
    0 => "zero"
    1 => "one"
    _ => "many"
}
```

## Lists

```claw
nums = [1, 2, 3]
nums.map(|n| n * 2)          # [2, 4, 6]
nums.len()                   # 3
```

## Blocks

Curly braces group a sequence of bindings ending in a result expression:

```claw
area = |w, h| {
    a = w * h
    a
}
```

## The entry point

Every runnable program defines `main!`:

```claw
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

```claw
main! = |_args| match risky() {
    Ok(v) => {
        echo!(v)
        Ok({})
    }
    Err(_) => Err(Exit(1))
}
```

## Where to go next

- [Getting started](getting-started.md) — install and project setup.
- [`examples/`](../examples) — hello, fizzbuzz, pattern matching, args.
- Let an agent write Claw for you: `claw mcp install` (see getting-started).
