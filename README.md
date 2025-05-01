# FiveM Hot Reload

Vous pouvez lire ce readme en français :

[![](https://img.shields.io/badge/Français-000?style=for-the-badge&logo=github&logoColor=white)](README.fr.md)

A tool for FiveM that allows you to hot reload resources.
This tool is composed of 3 parts:
- The Watcher: An executable that is placed at the root of your FiveM server (This root is where your `resources` folder is located).
- The Interface: A desktop application (stable on windows) and macos | linux (probably in the future) that allows you to manage profiles and receive resource change notifications.
- The Resource: A FiveM resource that allows you to receive resource change notifications and apply them hot.

## Features

### Interface
- Multi-profile management
- Default localhost profile (non-deletable) without API key
- Remote profiles with API key authentication
- Simplified configuration without resource folder reference
- Automatically fetched resource tree and watched files

### Watcher
- Real-time change detection
- Hot-reloading of changes
- Intelligent handling of fxmanifest modifications and file deletions/additions (to refresh before restarting the resource)
- Intelligent handling of only modified files (to restart without refresh)
- Integration of a CLI for internal commands (help, version, etc...)

### Resource
- Allows you to apply hot-reloaded changes detected by the watcher
- Requires permissions in the server.cfg
```
add_ace resource.hot-reload command.start allow
add_ace resource.hot-reload command.stop allow
add_ace resource.hot-reload command.ensure allow
add_ace resource.hot-reload command.refresh allow
add_ace resource.hot-reload command.add_ace allow
add_ace resource.hot-reload command.add_principal allow
```

## License

This project is under MIT license. See [LICENSE.txt](LICENSE.txt) file for more details.

## Contributors

You want to contribute to this project? Click on the badge below.

[![](https://img.shields.io/badge/-Contribution-000?style=for-the-badge&logo=github&logoColor=white)](CONTRIBUTING.md)

- [@sup2ak](https://github.com/sup2ak)

## Support

To report a bug or suggest an improvement, please open an issue on GitHub.