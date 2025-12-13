# Interfaces

Interfaces allow you to define a **contract** that classes must follow. An interface specifies *what* methods a class must implement, but not *how*.

## Defining an Interface

Use the `interface` keyword. Interface methods have no body and do not use curly braces.

```aegis
interface Printable {
    to_string()
    print_details(indent)
}
```

## Implementing an Interface

Use the `implements` keyword after the class name. You can implement multiple interfaces separated by commas.

```aegis
class User implements Printable {
    init(name) {
        this.name = name
    }

    // Must implement to_string
    to_string() {
        return "User: " + this.name
    }

    // Must implement print_details with 1 argument
    print_details(indent) {
        print indent + this.to_string()
    }
}
```

## Runtime Checks

Aegis verifies the contract when the class is loaded. If a method is missing or has the wrong number of arguments, the program will stop immediately with an error.

```aegis
interface Flyable {
    fly()
}

class Cat implements Flyable {
    // Error: Class 'Cat' must implement method 'fly' from interface 'Flyable'.
}
```

## Polymorphism

While Aegis is dynamically typed, interfaces help structure your code and ensure that objects passed around have the expected capabilities.

```aegis
func print_all(items) {
    // We assume items implement 'Printable'
    foreach (item in items) {
        item.print_details("-> ")
    }
}
```
