# Control Flow

Aegis provides standard control structures to direct the flow of your program.

## If / Else

Standard conditional logic. The condition must evaluate to a boolean or a truthy value.

```aegis
var hp = 50

if (hp > 75) {
    print "Healthy"
} else if (hp > 25) {
    print "Injured"
} else {
    print "Critical condition!"
}
```

## While Loop

Repeats a block of code as long as the condition is true.

```aegis
var i = 3
while (i > 0) {
    print "Countdown: " + i
    i = i - 1
}
print "Liftoff!"
```

## For Range Loop

Aegis uses a specific syntax for numeric iteration, optimized for the Virtual Machine.

Syntax: `for (variable, start, end, step)`

- `variable`: The name of the loop counter (automatically created).
- `start`: The starting value (inclusive).
- `end`: The stopping value (exclusive).
- `step`: The increment amount.

```aegis
// Print even numbers from 0 to 9
for (i, 0, 10, 2) {
    print "Number: " + i
}
```

## The Foreach Loop

The `foreach` loop provides a simplified syntax for iterating over collections like Lists or Strings. It automatically handles the index and bounds checking for you.

### Syntax

```aegis
foreach (variable in iterable) {
    // code block
}
```

### Iterating over Lists

```aegis
var users = ["Alice", "Bob", "Charlie"]

foreach (user in users) {
    print "Hello, " + user + "!"
}
```

### Iterating over Strings

You can also iterate over a string character by character.

```aegis
foreach (char in "Aegis") {
    print char
}
```

### Nesting

Foreach loops can be nested. The loop variable is local to its specific block, preventing conflicts.

```aegis
var matrix = [ [1, 2], [3, 4] ]

foreach (row in matrix) {
    foreach (cell in row) {
        print cell
    }
}
```

## Switch

The switch statement simplifies long `if`/`else` chains. Aegis switches perform an implicit break (no fall-through).

```aegis
var status = 200

switch (status) {
    case 200:
        print "OK"
    case 404:
        print "Not Found"
    case 500:
        print "Server Error"
    default:
        print "Unknown status"
}
```

## Loop Control: Break and Continue

You can finely control the execution of loops using `break` and `continue`.

### Break

Stops the loop immediately and resumes execution after the loop block.

```aegis
// Stop searching when we find the target
var target = 5
for (i, 0, 10, 1) {
    if (i == target) {
        print "Found it!"
        break
    }
}
```

Continue

Skips the rest of the current iteration and jumps directly to the next one (checking the condition in `while`, or incrementing in `for`).

```aegis
// Print only odd numbers
for (i, 0, 10, 1) {
    // If even, skip printing
    if (i % 2 == 0) { 
        continue 
    }
    print i
}
// Output: 1, 3, 5, 7, 9
```

## Ternary Operator

For simple conditions where you want to assign a value based on a check, the standard `if/else` can be verbose. Aegis provides the **Ternary Operator** `? :` for this purpose.

### Syntax

```aegis
condition ? value_if_true : value_if_false
```

### Example

Instead of:

```aegis
var status = null
if (age >= 18) {
    status = "Adult"
} else {
    status = "Minor"
}
```

You can write:

```aegis
var status = (age >= 18) ? "Adult" : "Minor"
```

### Nesting

Ternary operators can be nested, although this can reduce readability.

```aegis
var category = (score > 90) ? "A" : ((score > 50) ? "B" : "C")
```

## Null Coalescing Operator (`??`)

The null coalescing operator `??` is a logical operator that returns its right-hand side operand when its left-hand side operand is `null`, and returns its left-hand side operand otherwise.

It is cleaner and safer than using `||` because it only checks for `null`, not `false` or `0`.

### Syntax

```aegis
left_expr ?? default_value
```

### Examples

Basic Usage:

```aegis
var user_input = null
var username = user_input ?? "Guest"
print username // "Guest"
```

Difference with `||`:

```aegis
var count = 0

// || considers 0 as false
print count || 100 // 100 (Unwanted behavior?)

// ?? only cares about null
print count ?? 100 // 0 (Correct!)
```

### Chaining:

```
var config = null
var env = null
var port = config ?? env ?? 8080
print port // 8080
```
