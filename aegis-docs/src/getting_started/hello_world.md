# Hello World
Now that you have Aegis installed, it's time to write your first program. This is the traditional "Hello World" test to ensure everything is working correctly.

### 1. Create a Script

Create a new file named `hello.aeg` in your favorite text editor or IDE.

Add the following line to the file:

```aegis
print "Hello, World!"
```

### 2. Run the Script

Open your terminal, navigate to the folder where you saved the file, and run the following command:

```bash
aegis run hello.aeg
```

### 3. Output

You should see the following output in your terminal:

```
Hello, World!
```

If you see this message, congratulations! You have successfully compiled and executed your first Aegis program.

### Going Further: Interaction

Let's try something slightly more complex. Aegis has a built-in `input` instruction to interact with the user.

Update your `hello.aeg` file with the following code:

```aegis
print "--- Aegis Greeter ---"

// Ask for the user's name
input name "What is your name? "

// String concatenation
print "Welcome to Aegis, " + name + "!"
```

Run it again:

```bash
aegis run hello.aeg
```

Example interaction:

```
--- Aegis Greeter ---
What is your name? Ethan
Welcome to Aegis, Ethan!
```


What just happened?

- `print`: Displays text to the standard output (console).
- `input`: Pauses execution, waits for the user to type text and press Enter, and stores the result in the variable name.
- `+`: Joins (concatenates) the strings together.
