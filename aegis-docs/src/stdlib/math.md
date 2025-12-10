# Math &amp; Random

The Math module provides a collection of mathematical functions, constants, and a geometry class implemented directly in Aegis.

**Import:** `import "stdlib/math.aeg"`

## Constants

The module exposes common mathematical constants.

| Constant | Value | Description |
| :--- | :--- | :--- |
| `Math.PI` | `3.14159...` | Ratio of a circle's circumference to its diameter. |
| `Math.TAU` | `6.28318...` | Equal to `2 * PI`. |
| `Math.E` | `2.71828...` | Euler's number. |

## Basic Utilities

Helper functions for everyday logic.

| Function | Description |
| :--- | :--- |
| `Math.abs(n)` | Returns the absolute (positive) value of `n`. |
| `Math.max(a, b)` | Returns the larger of two numbers. |
| `Math.min(a, b)` | Returns the smaller of two numbers. |
| `Math.is_even(n)` | Returns `true` if `n` is even. |
| `Math.is_odd(n)` | Returns `true` if `n` is odd. |

## Arithmetic & Algebra

Advanced calculation functions.

### Power & Roots

* **`Math.pow(base, exp)`**: Calculates `base` raised to the power of `exp`.
* **`Math.sqrt(n)`**: Calculates the square root of `n` using the Newton-Raphson approximation method.
    * *Note:* Returns `-1` if `n` is negative.

### Number Theory

* **`Math.gcd(a, b)`**: Calculates the **Greatest Common Divisor** using the Euclidean algorithm.
* **`Math.lcm(a, b)`**: Calculates the **Least Common Multiple**.

```javascript
print Math.pow(2, 3)  // 8
print Math.sqrt(16)   // 4
print Math.gcd(12, 18) // 6
```

### Trigonometry

Functions to handle angles and waves. These implementations use Taylor Series approximations.

- `Math.sin(x)`: Sine of `x` (in radians).
- `Math.cos(x)`: Cosine of `x` (in radians).
- `Math.tan(x)`: Tangent of `x` (in radians).

### Conversions

- `Math.to_radians(deg)`: Converts degrees to radians.
- `Math.to_degrees(rad)`: Converts radians to degrees.

```
var angle = Math.to_radians(90)
print Math.sin(angle) // ~1.0
```

---

## Random Module

The Random module provides utilities for generating random numbers and selecting items from collections.

**Import:** `import "stdlib/random.aeg"`

### Number Generation

| Function | Description |
| :--- | :--- |
| `Random.int(min, max)` | Returns a random integer where `min` is inclusive and `max` is **exclusive**. |
| `Random.float()` | Returns a random floating-point number between `0.0` and `1.0`. |

### Collections

| Function | Description |
| :--- | :--- |
| `Random.choice(list)` | Returns a random element from the provided `list`. Returns `null` if the list is empty. |

### Example

```aegis
import "stdlib/random.aeg"

// 1. Roll a die (1 to 6)
var dice = Random.int(1, 7)
print "Dice roll: " + dice

// 2. Random probability
if (Random.float() < 0.5) {
    print "Heads!"
} else {
    print "Tails!"
}

// 3. Pick a winner
var players = ["Alice", "Bob", "Charlie"]
var winner = Random.choice(players)
print "The winner is: " + winner
```
