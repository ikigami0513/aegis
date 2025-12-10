# Import System

To organize code across multiple files, Aegis provides a powerful `import` statement.

## Syntax

```aegis
import "path/to/module.aeg"
```

The path is a string relative to the current working directory.

## How Imports Work

When you import a file:
- Execution: The VM loads, compiles, and executes the file immediately.
- Scope Sharing: The imported file runs in the same global scope as your main program.
- Caching: Aegis caches imports. If you import the same file twice, it will not be re-executed.

## Organizing with Namespaces

Since imports share the global scope, the best practice is to wrap library code in a `namespace` to keep things clean.

File: `lib/utils.aeg`

```aegis
namespace Utils {
    func hello() { return "Hello from module!" }
}
print "[Utils] Loaded."
```

File: `main.aeg`

```aegis
import "lib/utils.aeg"

// Using the namespace defined in the imported file
print Utils.hello()
```

Output:

```
[Utils] Loaded.
Hello from module!
```
