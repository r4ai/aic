# AIC: Experimental LLVM-Targeted Language via Inkwell

AIC is a simple, compiled programming language written in Rust that emits LLVM IR using the [Inkwell] library and compiles it down to an executable. It combines Rust‑style syntax with Go‑like semantics and is built for experimentation, not heavy optimization.

## Features

- Compiled directly to native executables via LLVM and [Inkwell]
- Lexer powered by [logos]
- Parser powered by [chumsky]
- Core syntax: `fn`, `let`, `if`, `else`, `for`, `while`, `return`, `mod` (WIP)
- Basic types: `i32`, `f64`, `bool`, `string` (WIP)
- Modules & scoped symbol resolution
- Simple standard library (I/O primitives)

For example code, see [`tests/fixtures`](tests/fixtures).

## Roadmap

Detailed multi‑phase roadmap in [`docs/roadmap.md`](docs/roadmap.md):

1. **Specification & Design**
2. **Lexer (logos) & Parser (chumsky)**
3. **AST Construction & Name/Type Checking**
4. **LLVM IR Code Generation (Inkwell)**
5. **Executable Generation & CLI**
6. **Tests & Documentation**
7. **Extensions & Optimizations**

## Getting Started

### Prerequisites

- Rust ≥ 1.86.0
- LLVM 18

  ```
  # For macOS or Linux, you can use Homebrew
  brew install llvm@18
  ```

### Build

```bash
cargo build --release
```

### Compile to LLVM IR

After building, you can compile an AIC source file using the following CLI options:

```
Usage: aic [OPTIONS] --input <INPUT>

Options:
  -i, --input <INPUT>    Input file to compile
  -o, --output <OUTPUT>  Output file
      --emit-llvm        Emit LLVM IR instead of an object file
  -h, --help             Print help
  -V, --version          Print version
```

#### Examples

- Compile to an object file:

  ```bash
  cargo run --release -- --input src/main.aic
  ```

  or with explicit output:

  ```bash
  cargo run --release -- --input src/main.aic --output main.o
  ```

- Emit LLVM IR to stdout:
  ```bash
  cargo run --release -- --input src/main.aic --emit-llvm
  ```

### Run

After compiling to a llvm object file, you can compile it to an executable using clang:

```bash
clang -o a.out main.o
```

Then run the executable:

```bash
./a.out
```

## Development

### Pre-requisites

1. Install LLVM 18:

   ```sh
   # For macOS or Linux, you can use Homebrew
   brew install llvm@18
   ```

2. Install tools for development:

   ```sh
   mise install
   ```

### Commands

| Command                       | Description                     |
| ----------------------------- | ------------------------------- |
| `mise tasks run build`        | Build the project               |
| `mise tasks run test`         | Run tests                       |
| `mise tasks run lint`         | Lint the project                |
| `mise tasks run lint-write`   | Lint and auto-fix the project   |
| `mise tasks run format`       | Check the project formatting    |
| `mise tasks run format-write` | Format and auto-fix the project |

[logos]: https://github.com/maciejhirsz/logos
[chumsky]: https://github.com/zesterer/chumsky
[Inkwell]: https://github.com/TheDan64/inkwell
