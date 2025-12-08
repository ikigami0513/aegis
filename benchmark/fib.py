import time

def fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)

print("--- PYTHON BENCHMARK (Fib 30) ---")

start = time.time()
result = fib(30)
end = time.time()

duration_ms = (end - start) * 1000

print(f"Resultat : {result}")
print(f"Temps : {duration_ms:.2f} ms")