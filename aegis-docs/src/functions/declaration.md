# Function Declaration

Functions are reusable blocks of code. In Aegis, functions are "first-class citizens," meaning they can be stored in variables, passed as arguments, and returned from other functions.

## Syntax

Use the `func` keyword followed by a name, parameters in parentheses, and a block of code.

```aegis
func greet(name) {
    print "Hello, " + name + "!"
}

// Calling the function
greet("Alice")
```

## Return Values

Use the `return` keyword to send a value back to the caller. If no return statement is provided, the function returns `null`.

```aegis
func add(a, b) {
    return a + b
}

var result = add(5, 10)
print result // 15
```

## Recursion

Functions can call themselves. Thanks to the stack-based VM, Aegis handles recursion efficiently.

```aegis
func fib(n) {
    if (n < 2) { return n }
    return fib(n - 1) + fib(n - 2)
}
```
