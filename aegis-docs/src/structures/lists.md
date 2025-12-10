# Lists

Lists are ordered collections of values. In Aegis, lists are dynamic (they can grow or shrink) and can hold values of mixed types.

## Creation

Use square brackets `[]` to create a list.

```aegus =
// Empty list
var empty = []

// Mixed types
var numbers = [1, 2, 3, 4]
var user = ["Admin", 42, true]
```

## Accessing Elements

Lists are 0-indexed. Use the `.at(index)` method to retrieve an element.

```aegis
var fruits = ["Apple", "Banana", "Cherry"]

print fruits.at(0) // Apple
print fruits.at(2) // Cherry

// Accessing an out-of-bounds index returns null
print fruits.at(99) // null
```

## Modifying Lists

You can add and remove elements dynamically.

| Method | Description | Example |
|--- |--- |--- |
| `.push(value)` | Adds an element to the end of the list. | `list.push(5)` |
| `.pop()` | Removes and returns the last element. | `var last = list.pop()` |
| `.len()` | Returns the number of elements. | `list.len()` |

### Example
```
var stack = []

stack.push("First")
stack.push("Second")

print stack.len() // 2

print stack.pop() // "Second"
print stack.len() // 1
```

*Note: Lists in Aegis are passed by reference. If you pass a list to a function and modify it there, the original list is affected.*
