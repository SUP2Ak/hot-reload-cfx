import WATCHER_API from "./class/app";
const RESOURCE_NAME = GetCurrentResourceName();

on('onResourceStart', (resourceName: string): void => {
    if (resourceName !== RESOURCE_NAME) return;
    console.log('^3Hot Reload Server started^0');
});

on('onResourceStop', (resourceName: string): void => {
    if (resourceName !== RESOURCE_NAME) return;
    console.log('^3Hot Reload Server stopped^0');
});

RegisterCommand('hot::status', (_: number): void => {
    console.log(`^3Hot Reload Server status: ${WATCHER_API.isRunning() ? '^2RUNNING' : '^1STOPPED'}^0`);
}, true);

RegisterCommand('hot::add', (_: number, args: string[]): void => {
    const resourceName = args[1];
    if (!resourceName) return console.log('^1Usage: hot::add <resourceName> | <resourceName.path.ext>^0');
    WATCHER_API.add(resourceName);
}, true);

RegisterCommand('hot::remove', (_: number, args: string[]): void => {
    const resourceName = args[1];
    if (!resourceName) return console.log('^1Usage: hot::remove <resourceName>^0');
    WATCHER_API.remove(resourceName);
}, true);

/**
 *  Ace Permissions:
 * 
 *  hot-reload.status
 *  hot-reload.add
 *  hot-reload.remove
 */