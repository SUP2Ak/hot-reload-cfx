# FiveM Hot Reload (Work In Progress)

Vous pouvez lire ce readme en français :

[![](https://img.shields.io/badge/Français-000?style=for-the-badge&logo=github&logoColor=white)](README.fr.md)

A cross-platform desktop application built with Rust to monitor and hot reload FiveM resources.

## Features (WIP)

- 🔄 Real-time resource monitoring
- 🚀 Automatic hot reload
- 📁 Resource file tree visualization
- 🌐 WebSocket communication
- 💻 Cross-platform support (Windows, Linux, MacOS)
- 🎨 Modern UI with egui

## Installation (Only when a release is available)

1. Download the latest release for your operating system:
   - Windows: hot-reload.exe
   - Linux: hot-reload
   - MacOS: hot-reload.app

Or build from source:

1. Make sure you have Rust installed
2. Clone this repository
3. Run: `cargo build --release`

## Usage

1. Launch the application
2. Click on "📂 Select Resources" to choose your FiveM resources folder
3. The application will automatically:
   - Scan for resources
   - Display the resource tree
   - Monitor for changes in .lua and .js files
   - Hot reload modified resources

## Configuration

The application creates a server_config.json file to store:

- Resources folder path
- WebSocket connection settings

## Technical Details

- Built with Rust and eframe/egui
- Uses tokio for async operations
- WebSocket communication for hot reload
- File system monitoring with notify
- Supports .lua and .js files and maybe later .net.dll

## Todo

- [ ]  Improve error handling
- [ ]  Add resource selection/deselection
- [ ]  Customize WebSocket connection settings
- [ ]  Review event handling
- [ ]  Add logging interface
- [ ]  Split into separate API service

## License

This project is licensed under the MIT License. See the [LICENSE.txt](LICENSE.txt) file for details.

## Contributors

- [@sup2ak](https://github.com/sup2ak)

## Issues

If you encounter any issues or have suggestions for improvements, please open an issue on the [GitHub repository](https://github.com/sup2ak/fivem-hot-reload/issues).

## Pull Requests

We welcome contributions to improve the project. Please see our [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to submit improvements and bug fixes.
