# Namespaces

As your code grows, you might run into naming conflicts (e.g., two functions named `init`). **Namespaces** allow you to group related variables and functions under a unique name.

## Defining a Namespace

Use the `namespace` keyword followed by a name and a block of code.

```aegis
namespace Math {
    var PI = 3.14159
    
    func square(x) {
        return x * x
    }
    
    func circle_area(r) {
        // Accessing siblings: explicit access is recommended
        return Math.PI * Math.square(r)
    }
}
```

## Accessing Members

You can access the contents of a namespace using the dot `.` notation, just like an object or a dictionary.

```aegis
print Math.PI // 3.14159

var area = Math.circle_area(10)
print area // 314.159
```

*Under the hood: In Aegis v0.2, a Namespace is compiled as a Dictionary containing the local variables defined in its scope.*