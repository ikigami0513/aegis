# Decorators

Decorators provide a clean syntax to modify or enhance the behavior of a function without changing its code. They are widely used for logging, access control, or performance measuring.

## Syntax

A decorator is simply a function that takes a function as an argument and returns a new function. You apply it using the `@` symbol.

```
// 1. Define the decorator
func logger(target_func) {
    return func(arg) {
        print "[LOG] Calling function with: " + arg
        var result = target_func(arg)
        print "[LOG] Result: " + result
        return result
    }
}

// 2. Apply it
@logger
func square(x) {
    return x * x
}

// 3. Use the decorated function
square(5)
```

Output:
```
[LOG] Calling function with: 5
[LOG] Result: 25
```

## How it works internally

The `@` syntax is syntactic sugar. The code above is equivalent to:

```aegis
func square(x) { return x * x }
square = logger(square)
```
