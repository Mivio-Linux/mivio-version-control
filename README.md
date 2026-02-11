# `MVC` (Mivio version control) is a utility written in Rust that provides a snapshot system that allows you to roll back code.
## Features:
1. Very lightweight: unlike similar projects, it's written in only ~250 lines of code (the binary file weighs 756 KB).
2. Easy to use: to work with MVC, you only need four commands (mvc [init, save <message>, return <id>, log]).
   
# Building from source

You’ll also need `cargo` to build from source:

```bash
cargo build -r
```
The binary will be located in `./target/release/`.

---

licensed under the MIT License 
