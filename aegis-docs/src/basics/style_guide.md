# Style Guide & Best Practices

To keep Aegis code clean and readable, we recommend following these conventions.

## Naming Conventions

* **Variables & Functions**: Use `snake_case`.
    ```aegis
    var my_variable = 10
    func calculate_total() { ... }
    ```

* **Classes**: Use `PascalCase`.
    ```aegis
    class GameManager { ... }
    ```

* **Constants**: Use `UPPER_SNAKE_CASE` inside Namespaces.
    ```aegis
    namespace Config {
        var MAX_RETRIES = 5
    }
    ```

## File Structure

* Use `.aeg` extension for all scripts.
* Place reusable code in a `lib/` or `modules/` folder.
* Wrap library code in a `namespace` to avoid polluting the global scope.

## Error Handling

Prefer `try/catch` over checking for `null` when performing I/O operations (File, Network).

```aegis
// ✅ Good
try {
    var data = File.read("config.json")
} catch (e) {
    print "Config not found, using defaults."
}

// ❌ Avoid
var data = File.read("config.json") // If fails, script crashes!
```
