# Sample Project

A sample project structure for testing `.xunbak` container backup and restore.

## Features

- Source code files (`.rs`) for compression testing
- Configuration files (`.json`, `.toml`)
- Documentation in Chinese and English
- Binary/pre-compressed file simulation
- Deep nested directory structures
- Files with special path characters

## Getting Started

```bash
cargo build
cargo test
cargo run
```

## Directory Structure

```
src/           - Source code
config/        - Configuration files
docs/          - Documentation
assets/        - Binary assets (images, archives)
中文目录/      - Chinese path testing
path with spaces/ - Space in path testing
deep/          - Deep nesting testing
empty_dir/     - Empty directory testing
```

## License

MIT
