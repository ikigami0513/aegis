# Testing Framework

Aegis includes a lightweight unit testing framework to help you write reliable code.

Import: `import "stdlib/test.aeg"`

## Writing Tests

Use `Test.run` to define a test case. It creates a protected scope where errors are caught.

```javascript
Test.run("Calculations", func() {
    var sum = 10 + 20
    Assert.eq(sum, 30, "Sum should be 30")
})
```

## Assertions

The `Assert` namespace provides methods to validate conditions. If an assertion fails, the test stops and is marked as failed.

| Method | Description |
|--- |--- |
| `Assert.eq(a, b, msg)` | Fails if `a != b`. Prints `msg` on failure. |
| `Assert.is_true(cond, msg)` | Fails if the condition is false. |

*Note: We use `is_true` instead of `true` to avoid conflict with the boolean keyword.*

Example Output

```
TEST: Calculations...
  ✅ PASS
TEST: Bad Math...
  ❌ FAIL: Assertion Failed: 10 != 5 (Math is broken)
```
