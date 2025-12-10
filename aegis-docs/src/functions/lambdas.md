# Lambdas &amp; Closures

Aegis supports anonymous functions, often called **Lambdas**. These are functions without a name, typically used for short operations or callbacks.

## Creating a Lambda

The syntax is identical to a standard function, but without the name.

```aegis
var say_hello = func() {
    print "Hello from Lambda!"
}

// Execute the variable
say_hello()
```

## Closures (Capturing Environment)

Lambdas in Aegis are Closures. This means they can "capture" and remember variables from the scope in which they were defined, even after that scope has finished executing.

```aegis
func make_counter() {
    var count = 0
    
    // This lambda captures 'count'
    return func() {
        count = count + 1
        return count
    }
}

var counter = make_counter()

print counter() // 1
print counter() // 2
print counter() // 3
```

*Note: Aegis v0.2 captures variables by value (snapshot) or by reference depending on implementation specifics. In the current version, complex logic inside closures is fully supported.*