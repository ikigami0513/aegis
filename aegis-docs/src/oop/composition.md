# Composition Pattern

Aegis promotes flexible design patterns. While inheritance creates rigid hierarchies ("is-a" relationship), **Composition** allows you to build complex objects by combining simpler ones ("has-a" relationship).

## Concept

Instead of a class inheriting from another, it holds an instance of another class as a property.

## Example: Game Entity

Imagine building a game. Instead of a deep inheritance tree, you can compose an `Entity` using a `Position` and a `Stats` object.

```aegis
// Component 1: Position
class Vector(x, y) {
    func str() { return "(" + this.x + ", " + this.y + ")" }
}

// Component 2: Stats
class Stats(hp, mana) {
    func is_alive() { return this.hp > 0 }
}

// Main Entity using Composition
class Player(name, x, y) {
    // We initialize components in the constructor logic (class body)
    
    this.pos = new Vector(x, y)
    this.stats = new Stats(100, 50)
    
    func info() {
        print this.name + " is at " + this.pos.str()
    }
    
    func take_damage(amount) {
        this.stats.hp = this.stats.hp - amount
    }
}
```

Usage

```
var p = new Player("Hero", 10, 10)

p.info() // Hero is at (10, 10)

p.take_damage(20)
print "HP Left: " + p.stats.hp // 80
```

This approach makes your code modular and easier to test.
