# Vo Benchmark Suite

Performance comparison between **Vo** (VM & JIT), **Go**, **Lua/LuaJIT**, and **Python**.

## Prerequisites

```bash
# macOS
brew install hyperfine go lua luajit python3
```

## Usage

```bash
# Run all benchmarks
./benchmark/run.sh

# Run specific benchmark
./benchmark/run.sh fibonacci
./benchmark/run.sh binary-trees
./benchmark/run.sh sieve

# List available benchmarks
./benchmark/run.sh list
```

## Benchmarks

| Benchmark | Description | Tests |
|-----------|-------------|-------|
| **fibonacci** | Recursive Fibonacci (n=35) | Function call overhead, recursion |
| **binary-trees** | Allocate/deallocate binary trees (depth=18) | GC pressure, memory allocation |
| **sieve** | Sieve of Eratosthenes (n=1M) | Array access, loops, conditionals |

## Results

Results are exported to `benchmark/results/`:
- `*.json` - Raw hyperfine JSON output
- `*.md` - Markdown table

## Adding New Benchmarks

1. Create directory: `benchmark/<name>/`
2. Add implementations:
   - `<name>.vo` - Vo implementation
   - `<name>.go` - Go implementation
   - `<name>.lua` - Lua implementation
   - `<name>.py` - Python implementation
3. Run: `./benchmark/run.sh <name>`

## Notes

- **Vo-VM**: Bytecode interpreter
- **Vo-JIT**: Cranelift JIT compiler (with `VO_JIT_CALL_THRESHOLD=1` for immediate compilation)
- **Go**: Native compiled
- **LuaJIT**: JIT compiled (if available, otherwise plain Lua)
- **Python**: CPython interpreter

## Example Output

```
=== fibonacci ===

Benchmark 1: Vo-VM
  Time (mean ± σ):     1.234 s ±  0.012 s

Benchmark 2: Vo-JIT
  Time (mean ± σ):     0.456 s ±  0.008 s

Benchmark 3: Go
  Time (mean ± σ):     0.123 s ±  0.002 s

Benchmark 4: LuaJIT
  Time (mean ± σ):     0.234 s ±  0.005 s

Benchmark 5: Python
  Time (mean ± σ):     5.678 s ±  0.045 s

Summary
  Go ran
    1.85 ± 0.04 times faster than LuaJIT
    3.70 ± 0.07 times faster than Vo-JIT
   10.03 ± 0.15 times faster than Vo-VM
   46.17 ± 0.52 times faster than Python
```
