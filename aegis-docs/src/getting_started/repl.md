# The REPL

### Status Note (v0.2.0)

The Interactive REPL is now functional with the new `v0.2.0` Bytecode Virtual Machine.

## What is a REPL?

REPL stands for Read-Eval-Print Loop. It is an interactive shell that allows you to type Aegis code and see the results immediately, without creating a file.

It is typically used for:
- Testing small snippets of logic.
- Doing quick calculations.
- Exploring the available Standard Library functions.

## Starting a Session

To enter the interactive mode, run the repl command:

```bash
aegis repl
```

Or simply run the executable without arguments:

```bash
aegis
```

You will see the prompt >> waiting for your input.

## Usage Example

Concept of a typical session once the migration is complete:

```bash
Aegis v0.2.0 - Interactive Mode
Type 'exit' or 'quit' to leave.

>> var a = 10
>> var b = 5
>> print a * b
50

>> func greet(name) { return "Hello " + name }
>> print greet("User")
Hello User
```

## Exiting

To exit the REPL and return to your system terminal, type:

```
exit
```

Or `quit`.