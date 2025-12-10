# Functional Programming

Aegis provides built-in methods on **Lists** to process data using a functional style. This allows for cleaner, more expressive code compared to traditional loops.



## Map
Syntax: `map(callback)`

Creates a new list by applying a function to every element in the original list.

```aegis
var numbers = [1, 2, 3, 4]

var doubled = numbers.map(func(n) {
    return n * 2
})

print doubled // [2, 4, 6, 8]
```

## Filter
Syntax: `filter(callback)`

Creates a new list containing only elements for which the callback returns true.

```aegis
var numbers = [10, 5, 20, 3]

var big = numbers.filter(func(n) {
    return n > 8
})

print big // [10, 20]
```

## For each
Syntax: `for_each(callback)`

Executes a function for every element in the list. Useful for side effects (like printing or saving).

```aegis
var names = ["Alice", "Bob"]

names.for_each(func(name) {
    print "User: " + name
})
```

## Chaining

Since map and filter return new Lists, you can chain them together.

```aegis
// Take 1..5, multiply by 10, keep those > 20
var res = [1, 2, 3, 4, 5]
    .map(func(n) { return n * 10 })
    .filter(func(n) { return n > 20 })

print res // [30, 40, 50]
```
