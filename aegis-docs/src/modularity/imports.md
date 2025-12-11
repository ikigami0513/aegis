# Import System

To organize code across multiple files, Aegis provides a powerful `import` statement.

## Syntax

```aegis
var module = import "path/to/module.aeg"
```

The path is a string relative to the current working directory.

## How Imports Work

When you import a file:
- **Execution**: The VM loads, compiles, and executes the file immediately.
- **Scope Sharing**: The imported file shares the global scope (native functions, etc.).
- **Return Value**: The `import` expression returns the last value evaluated in the script (or `null` if nothing is returned).
- **Caching**: Aegis caches the returned value. If you import the same file twice, it is not re-executed; the cached value is returned immediately.

## Pattern 1: Global Inclusion (Legacy)

In this pattern, the imported file defines a Namespace or variables directly in the global scope.

**File**: `lib/utils.aeg`

```aegis
namespace Utils {
    func hello() { return "Hello Global!" }
}
```

**File**: `main.aeg`

```
import "lib/utils.aeg" // Returns null, but 'Utils' is now defined globally
print Utils.hello()
```

## Pattern 2: The Module Pattern (Recommended)

To avoid naming conflicts (e.g., two libraries defining a `Common` namespace), it is best practice to return the namespace at the end of the file and assign it to a variable.

**File**: `lib/math_v1.aeg`

```aegis
namespace Math {
    func add(a, b) { return a + b }
}
// Export the namespace
return Math
```

**File**: `lib/math_v2.aeg`

```aegis
namespace Math {
    func add(a, b) { return a + b + 0.5 }
}
return Math
```

**File**: `main.aeg`

```aegis
// We can now load two modules that have the same internal name
var M1 = import "lib/math_v1.aeg"
var M2 = import "lib/math_v2.aeg"

print M1.add(10, 10) // 20
print M2.add(10, 10) // 20.5
```

This pattern ensures your code remains modular and safe from global scope pollution.
