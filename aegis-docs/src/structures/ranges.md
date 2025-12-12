# Ranges

Ranges represent a sequence of numbers. Unlike Lists, Ranges are **lazy**: they do not store all the numbers in memory. Instead, they calculate the next number on the fly. This makes them extremely efficient for loops.

## Syntax

Use the `..` operator to create a range.

```aegis
var r = 0..10
```

- Start: Inclusive.
- End: Exclusive (the number after .. is not included).

## Usage in Loops

Ranges are primarily used with `foreach` loops.

```
// Prints 0, 1, 2, 3, 4
foreach (i in 0..5) {
    print i
}
```

## Methods

Ranges are objects and expose several methods to manipulate the sequence.

|Method | Description | Example |
|--- |--- |--- |
| `.step(n)` | Returns a new Range with a specific step. | `(0..10).step(2)` |
| `.to_list()` | Converts the Range into a standard List. | `(1..4).to_list()` -> `[1, 2, 3]` |
| `.len()` | Calculates the number of elements in the range. | `(0..10).step(2).len()` -> `5` |
| `.at(index)` |Returns the number at the Nth step. | `(0..10).at(2)` -> `2` |

## Custom Steps and Reverse Loops

By default, the step is `1`. You can change it using `.step()`.

```aegis
// Even numbers: 0, 2, 4, 6, 8
foreach (i in (0..10).step(2)) {
    print i
}

// Countdown: 10, 9, 8 ... 1
foreach (i in (10..0).step(-1)) {
    print "T-minus " + i
}
```
