# FiveM Hot Reload (Work In Progress)

Vous pouvez lire ce readme en français :

[![](https://img.shields.io/badge/Français-000?style=for-the-badge&logo=github&logoColor=white)](README.fr.md)

A cross-platform desktop application built with Rust to monitor and hot reload FiveM resources.

## Current Features

### Profile System
- Multi-profile connection management
- Default localhost profile (non-removable) without API key
- Remote profiles with API key authentication
- Simplified configuration without resources folder referencing

### Architecture
- Clear separation between UI (client) / Watcher (server)
- Integrated API key generator
- Standalone watcher to place at server root
- Automatic configuration on first launch

### Communication
- Secure WebSocket for remote connections
- Automatic authentication based on profile type
- Real-time change detection

## In Development

### User Interface
- [ ] Checkbox system to ignore/watch folders and files
- [ ] Logs interface (watcher, application, resources)
- [ ] User experience improvements
- [ ] Advanced profile management

### Watcher
- [ ] Finalization of `handle_change`
- [ ] Smart handling of fxmanifest modifications
- [ ] Detection and processing of added/removed resources
- [ ] Performance optimization

### FiveM Resource
- [ ] Improvement of internal commands execution
- [ ] Detailed logs interface
- [ ] Enhanced error handling

## Installation

1. Download the latest version
2. For server: place the watcher at your FiveM server root
3. For client: launch the UI application
4. Configure your profiles according to your needs

## Usage

1. Start the watcher on your server
2. Launch the client interface
3. Select or create a profile
4. Connect and start developing

## License

This project is under MIT license. See [LICENSE.txt](LICENSE.txt) file for more details.

## Contributors

- [@sup2ak](https://github.com/sup2ak)

## Support

To report a bug or suggest an improvement, please open an issue on GitHub.