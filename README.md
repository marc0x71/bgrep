# bgrep
[![Rust CI](https://github.com/marc0x71/bgrep/actions/workflows/ci.yml/badge.svg)](https://github.com/marc0x71/bgrep/actions/workflows/ci.yml) ![License](https://img.shields.io/badge/license-MIT-blue)
  
**bgrep** is a fast command-line search tool for sorted, delimited text files. Written in Rust, it uses **binary search** to locate records in **O(log n)** time — making it orders of magnitude faster than `grep` on large files where the data is already sorted.

> **When to use bgrep vs grep:**
> Use `grep` for arbitrary text searches on unsorted data. Use `bgrep` when your file is sorted on a known column and you need to search it repeatedly or at scale.

---

## Requirements

- Linux or macOS
- A text file that is **sorted** on the column you want to search

---

## Installation

```bash
cargo install --path .
```

---

## Usage

```
bgrep [OPTIONS] <TARGET> <FILE>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `TARGET` | The value to search for |
| `FILE`   | Path to the sorted delimited text file |

### Options

| Option | Long form | Default | Description |
|--------|-----------|---------|-------------|
| `-k <N>` | `--key <N>` | `0` | Index of the column to search on (0-based) |
| `-d <C>` | `--delimiter <C>` | `;` | Field delimiter character |
| `-n` | `--numeric` | false | Treat the key column as a number |

---

## Examples

Given a file `users.csv` with semicolon-separated fields, sorted by username (column 0):

```
alice;28;engineer
bob;34;designer
carol;25;manager
```

**Search by default key (column 0):**
```bash
bgrep bob users.csv
```
```
bob;34;designer
```

**Search on a different column** — file sorted by age (column 1):
```bash
bgrep -k 1 -n 34 users.csv
```
```
bob;34;designer
```

**Use a comma as delimiter:**
```bash
bgrep -d , -k 2 engineer users.csv
```

**Search for a numeric key:**
```bash
bgrep -n -k 1 25 users.csv
```

**Multiple matches** — if the file contains multiple rows with the same key, all are printed:
```bash
bgrep -k 2 engineer users.csv
```
```
alice;28;engineer
dave;31;engineer
```

---

## Performance

| Tool | Time complexity | Best for |
|------|----------------|----------|
| `grep` | O(n) — scans every line | Unsorted files, pattern matching |
| `bgrep` | O(log n) — binary search | Large sorted files, exact key lookup |

Benchmarks on a 10-million-line file (values in range 0–1,000,000):

| Scenario | `grep` | `bgrep` | Speedup |
|----------|--------|---------|---------|
| Value near the start of file | 93ms | 72ms | ~1.3x |
| Value near the end of file | 127ms | 3ms | **~42x** |
| Value not present in file | 119ms | 3ms | **~40x** |

The key insight: **bgrep's cost is constant regardless of where the value is** (or whether it exists at all). `grep` always pays the full O(n) price — it must scan every line to confirm a value is absent. `bgrep` performs at most ~20 seeks on 10M lines no matter what.

The advantage is modest when the value appears near the beginning of the file. It becomes decisive for values near the end or absent from the file — exactly the cases where `grep` is slowest.

---

## Important Notes

- The file **must be sorted** on the key column. Results are undefined otherwise.
- Sorting must be **lexicographic** by default, or **numeric** when using `-n`.
- Column indices are **0-based** (first column is `0`).
- The delimiter must be a **single character**.
- The numeric mode (`-n`) supports **integers** only (no decimals).

### Preparing a sorted file

If your file is not yet sorted, you can sort it with the standard `sort` utility before using `bgrep`:

```bash
# Sort lexicographically on column 0 (semicolon-delimited)
sort -t ';' -k1,1 input.csv > sorted.csv

# Sort numerically on column 1
sort -t ';' -k2,2n input.csv > sorted.csv
```

---

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Match found |
| `1` | No match found |
| `2` | Error (file not found, invalid arguments, etc.) |

This follows the same convention as `grep`, making `bgrep` easy to use in scripts:

```bash
if bgrep -n 999 data.csv > /dev/null; then
    echo "found"
else
    echo "not found"
fi

# or with &&/||
bgrep -n 999 data.csv && echo "exists"
```
