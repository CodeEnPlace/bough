+++
title = "Getting Started"
idx = 1
+++

Install bough and run your first mutation test.

## Installation

```bash
cargo install bough
```

## Configuration

Create a `bough.toml` in your project root:

```toml
[language.javascript]
include = ["src/**/*.js"]
test_cmd = "npm test"
```

## Usage

```rust
fn main() {
    let x = 1 + 2;
    println!("{x}");
}
```
