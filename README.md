# 🎡 Turntable

Turntable is an implementation of the unpkg backend interface for the Rust version. It is based on Poem and Tokio.

## 🚀 Getting Started

### Prerequisites

- Rust 1.65 or higher
- Cargo

### Installation

1. Clone the repository:

   ```
   git clone https://github.com/Binbiubiubiu/turntable.git
   ```

2. Build the project:

   ```
   cargo build
   ```

3. Run the project:

   ```
   cargo run
   ```

## 📝 Usage

Turntable provides an API that allows you to access the unpkg backend interface. You can use it to fetch and serve JavaScript packages.

### Endpoints

- `/` - Returns the homepage.
- `/package/:name@:version/:file` - Returns a specific file from a package.
- `/package/:name@:version/:file?meta` - Returns metadata about a specific file from a package.

### Examples

- Fetch a specific file from a package:

  ```
  curl http://localhost:8000/package/lodash@4.17.21/lodash.js
  ```

- Fetch metadata about a specific file from a package:

  ```
  curl http://localhost:8000/package/lodash@4.17.21/lodash.js?meta
  ```

## 🤝 Contributing

Contributions are welcome! Feel free to open an issue or submit a pull request.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
