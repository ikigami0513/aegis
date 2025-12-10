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
