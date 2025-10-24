# Benchmarking Guide

This document describes the benchmark suite for the ANSI parser and how to run and interpret the results.

## Running Benchmarks

### Basic Usage

Run all benchmarks:
```bash
cargo bench
```

Run specific benchmark group:
```bash
cargo bench plain_text
cargo bench sgr_sequences
cargo bench mixed_content
```

Run with baseline for comparison:
```bash
# Save current performance as baseline
cargo bench -- --save-baseline main

# After making changes, compare against baseline
cargo bench -- --baseline main
```

### Output

Benchmarks will generate:
- Console output with timing statistics
- HTML reports in `target/criterion/` directory
- Comparison graphs if comparing against a baseline

## Benchmark Groups

### 1. `plain_text`
Tests parsing performance on pure text without any escape sequences.
- **Sizes**: 100, 1K, 10K bytes
- **Purpose**: Establish baseline for fast-path performance
- **Expected**: Very fast, should use memchr optimization

### 2. `plain_text_newlines`
Tests text with newlines (common in terminal output).
- **Sizes**: 10, 100, 1K lines
- **Purpose**: Test newline handling overhead
- **Expected**: Slightly slower than pure text due to newline processing

### 3. `colored_output`
Simulates `ls --color` style output with basic SGR sequences.
- **Sizes**: 10, 100, 1K lines
- **Purpose**: Test realistic colored terminal output
- **Expected**: Moderate speed, tests SGR parsing

### 4. `sgr_sequences`
Tests different SGR (color/style) sequence types:
- `simple`: Basic colors (e.g., `\x1B[31m`)
- `complex`: Multiple attributes (e.g., `\x1B[1;4;31;44m`)
- `256color`: 256-color mode (e.g., `\x1B[38;5;196m`)
- `truecolor`: RGB colors (e.g., `\x1B[38;2;255;128;0m`)
- **Purpose**: Compare SGR parsing overhead by complexity
- **Expected**: Simple < Complex < 256color < Truecolor

### 5. `cursor_movement`
Tests cursor positioning sequences:
- `simple`: Basic movement (A, B, C, D)
- `absolute`: Direct positioning (H)
- `save_restore`: Cursor save/restore
- `complex`: Multiple movements
- **Purpose**: Test cursor operation overhead
- **Expected**: All should be fast

### 6. `mixed_content`
Realistic shell session output with colors, text, and formatting.
- **Sizes**: 10, 100, 1K repetitions
- **Purpose**: Test real-world performance
- **Expected**: Representative of actual usage

### 7. `osc_sequences`
Tests OSC (Operating System Command) sequences:
- `title`: Window title setting
- `title_long`: Long window titles
- `hyperlink`: Hyperlink sequences
- **Purpose**: Test OSC parsing overhead
- **Expected**: Moderate, depends on string operations

### 8. `utf8_content`
Tests UTF-8 parsing with different character types:
- `ascii`: Pure ASCII (baseline)
- `latin`: Latin extended characters
- `chinese`: CJK characters
- `emoji`: Emoji (multi-byte)
- `mixed`: Mix of all above
- **Purpose**: Verify UTF-8 handling performance
- **Expected**: ASCII fastest, emoji slightly slower

### 9. `worst_case`
Stress tests for potentially slow scenarios:
- `many_params`: CSI with many parameters
- `long_osc`: Very long OSC sequences
- `rapid_sgr`: Rapid SGR changes
- **Purpose**: Identify performance bottlenecks
- **Expected**: Should still be reasonable, no exponential behavior

### 10. `parser_reuse`
Tests parser reuse (no allocation overhead).
- **Purpose**: Verify efficient parser reuse
- **Expected**: Fast, minimal allocation

### 11. `streaming_chunks`
Tests streaming input in different chunk sizes.
- **Chunk sizes**: 10, 100, 1K bytes
- **Purpose**: Test streaming behavior
- **Expected**: Similar throughput regardless of chunk size

## Performance Goals

### Target Performance (on modern CPU)

- **Plain text**: >100 MB/s
- **Colored output**: >50 MB/s
- **Mixed content**: >30 MB/s
- **UTF-8 text**: >80 MB/s

### Relative Performance

- Simple SGR: ~90% of plain text speed
- Complex SGR: ~70% of plain text speed
- Cursor movement: ~85% of plain text speed
- OSC sequences: ~60% of plain text speed

## Profiling

For detailed profiling, use `cargo flamegraph`:

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph for specific benchmark
cargo flamegraph --bench parser_bench -- --bench plain_text
```

Or use `perf` on Linux:

```bash
# Build with debug symbols
cargo bench --no-run

# Find the benchmark binary
BENCH_BIN=$(find target/release/deps -name "parser_bench-*" -type f)

# Run with perf
perf record --call-graph=dwarf $BENCH_BIN --bench
perf report
```

## Continuous Monitoring

To track performance over time:

1. Save baseline for each release:
   ```bash
   cargo bench -- --save-baseline v0.1.0
   ```

2. Compare against previous version:
   ```bash
   cargo bench -- --baseline v0.1.0
   ```

3. Document any significant changes (>10%) in CHANGELOG.md

## Optimization Tips

If benchmarks show performance issues:

1. **Check for allocations**: Use `cargo bench --profile bench` with profiling tools
2. **Look at flamegraphs**: Identify hot paths
3. **Compare baselines**: Ensure changes don't regress performance
4. **Focus on real-world cases**: Don't over-optimize synthetic benchmarks

## CI Integration

Add to CI to prevent performance regressions:

```yaml
# .github/workflows/bench.yml
- name: Run benchmarks
  run: |
    cargo bench -- --save-baseline current
    cargo bench -- --baseline current
```

## Interpreting Results

Example output:
```
plain_text/100          time:   [245.23 ns 247.89 ns 250.84 ns]
                        thrpt:  [398.62 MiB/s 403.42 MiB/s 407.82 MiB/s]
```

- **time**: Mean execution time with confidence interval
- **thrpt**: Throughput (higher is better)
- **Change**: Percent change from baseline (if comparing)

Look for:
- ✅ Consistent timing across runs (narrow confidence interval)
- ✅ Linear scaling with input size
- ⚠️ Large variations suggest unstable benchmarks
- ⚠️ Non-linear scaling suggests algorithmic issues

## Benchmark Maintenance

- Run benchmarks before major releases
- Update baselines when making intentional performance changes
- Document performance characteristics in docs
- Add new benchmarks for new features