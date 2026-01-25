# d.py

```bash
./d.py test [both|vm|jit|gc|nostd|wasm] [-v|--verbose] [--arch=32|64] [--direct] [file_or_dir]
./d.py bench [all|vo|score|<name>] [--all-langs] [--arch=32|64]
./d.py loc [--with-tests]
./d.py clean [all|vo|rust]
./d.py play [--build-only]
./d.py run <file.vo> [--mode=vm|jit] [--codegen]
```

# vo (cmd/vo)

```bash
cargo run --bin vo -- cmd/vo run <file|dir> [--mode=jit] [--ast] [--codegen]
cargo run --bin vo -- cmd/vo build [path]
cargo run --bin vo -- cmd/vo check [path]
cargo run --bin vo -- cmd/vo dump <file.vob|file.vot>
cargo run --bin vo -- cmd/vo compile <file.vot> [-o out.vob]
cargo run --bin vo -- cmd/vo init <module-path>
cargo run --bin vo -- cmd/vo get <module@version>
cargo run --bin vo -- cmd/vo help
cargo run --bin vo -- cmd/vo version
```
