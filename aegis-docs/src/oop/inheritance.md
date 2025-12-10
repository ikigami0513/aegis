# Inheritance

Inheritance is a fundamental concept in Object-Oriented Programming that allows a class (the **Child**) to derive behavior and properties from another class (the **Parent**).

In Aegis, inheritance is achieved using the `extends` keyword.



## Basic Inheritance

When a class extends another, it automatically gains access to all methods defined in the parent class.

```aegis
// The Parent Class
class Animal(name) {
    speak() {
        print this.name + " makes a generic noise."
    }
    
    sleep() {
        print this.name + " is sleeping."
    }
}

// The Child Class
class Dog(name) extends Animal {
    // Dog creates its own scope but inherits 'speak' and 'sleep'
    
    fetch() {
        print this.name + " runs after the ball!"
    }
}

var d = new Dog("Rex")

d.fetch() // "Rex runs after the ball!" (Defined in Dog)
d.speak() // "Rex makes a generic noise." (Inherited from Animal)
```

## Method Overriding

A child class can provide its own implementation of a method that already exists in the parent class. This is called Overriding.

When you call a method, Aegis looks for it in the current class first. If found, it executes it. If not, it looks in the parent class.

```aegis
class Cat(name) extends Animal {
    // Overriding the 'speak' method
    speak() {
        print "Meow!"
    }
}

var c = new Cat("Luna")
c.speak() // "Meow!" (The parent's method is ignored)
c.sleep() // "Luna is sleeping." (Still uses the parent's method)
```

## The Prototype Chain

Aegis supports multi-level inheritance. A class can inherit from a class that inherits from another class.

```aegis
class LivingBeing(age) { 
    is_alive() { return true } 
}

class Animal(name, age) extends LivingBeing { 
    // ... 
}

class Dog(name, age, breed) extends Animal { 
    // ... 
}

var d = new Dog("Buddy", 5, "Golden")

// Dog -> Animal -> LivingBeing -> Found!
print d.is_alive() // true
```

*Note: Currently, Aegis supports Single Inheritance (a class can only extend one parent).*

## Accessing Parent Methods (`super`)

When you override a method, you often want to **extend** the parent's behavior rather than replacing it entirely. You can explicitly call a method from the parent class using the `super` keyword.

### Syntax

To call a method from the parent class inside a child class method:

```aegis
super.methodName(arguments)
```

### Example

In this example, the `Hero` class overrides `init` but still calls `Entity`'s `init` to ensure the base setup is done.

```aegis
class Entity(name) {
    init() {
        print "Entity initialized: " + this.name
    }
    
    speak() {
        return "..."
    }
}

class Hero(name, hp) extends Entity {
    init() {
        // 1. Call the parent method first
        super.init()
        
        // 2. Add child-specific logic
        print "Hero ready with " + this.hp + " HP"
    }
    
    speak() {
        // Reuse parent result in the new string
        return "Hero says: " + super.speak() + " (Ready!)"
    }
}

var h = new Hero("Link", 100)
h.init()
print h.speak()
```

Output:

```
Entity initialized: Link
Hero ready with 100 HP
Hero says: ... (Ready!)
```

This pattern is essential for avoiding code duplication when building complex class hierarchies.