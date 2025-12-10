# Error Handling

Scripts can fail (missing files, network errors, bad math). Aegis provides a robust `try/catch` mechanism to handle these situations gracefully instead of crashing the Virtual Machine.

## Try / Catch

Wrap risky code in a `try` block. If an error occurs, execution immediately jumps to the `catch` block.

```
try {
    print "1. Doing something risky..."
    var result = 10 / 0 // Division by zero error
    print "2. This will not run."
} catch (error) {
    print "3. Error caught: " + error
}

print "4. Program continues."
```

Output:

```
1. Doing something risky...
3. Error caught: Division by zero
4. Program continues.
```

## Throwing Errors

You can raise your own errors using the `throw` keyword. You can throw strings or any other value.

```aegis
func validate_age(age) {
    if (age < 0) {
        throw "Age cannot be negative!"
    }
    return true
}

try {
    validate_age(-5)
} catch (e) {
    print "Validation failed: " + e
}
```

*Note: Aegis native modules (like File or Http) throw exceptions when operations fail. You should wrap I/O operations in try/catch blocks.*