# AIC: Experimental LLVM-Targeted Language via Inkwell

AIC is a simple, compiled programming language written in Rust that emits LLVM IR using the [Inkwell] library and compiles it down to an executable. It combines Rust‑style syntax with Go‑like semantics and is built for experimentation, not heavy optimization.

## Features

- Compiled directly to native executables via LLVM and [Inkwell]
- Lexer powered by [logos]
- Parser powered by [chumsky]
- Core syntax: `fn`, `let`, `if`, `else`, `for`, `while`, `return`, `mod`
- Basic types: `i32`, `f64`, `bool`, `string`
- Modules & scoped symbol resolution
- Simple standard library (I/O primitives)

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

- Rust ≥ 1.60
- LLVM development libraries (refer to [Inkwell] documentation for specific requirements based on your OS)
  ```bash
  # Example for Ubuntu/Debian:
  # sudo apt-get install llvm-dev libclang-dev clang
  ```
- (Optional) [lld] for linking, or use the system linker.

### Build

```bash
cargo build --release
```

### Compile to Executable

```bash
# example: compile src/main.aic to an executable 'a.out' (default)
cargo run --release -- build src/main.aic -o a.out

# example: compile src/main.aic to out.ll (LLVM IR only, for debugging)
cargo run --release -- build src/main.aic -o out.ll --emit=llvm-ir
```

### Run

```bash
# Run the compiled executable
./a.out
```

## Testing

```bash
cargo test
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

## Contributing

Contributions welcome! See [`docs/roadmap.md`](docs/roadmap.md) for planned features and phases. Please open issues or PRs for bugs and improvements.

[logos]: https://github.com/maciejhirsz/logos
[chumsky]: https://github.com/zesterer/chumsky
[Inkwell]: https://github.com/TheDan64/inkwell
[lld]: https://lld.llvm.org/
