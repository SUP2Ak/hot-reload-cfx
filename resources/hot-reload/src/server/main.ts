import WATCHER_API from "./class/app";

on('onResourceStart', (resourceName: string): void => {
    if (resourceName !== GetCurrentResourceName()) return;
    console.log('^3Hot Reload Server started^0');
});

on('onResourceStop', (resourceName: string): void => {
    if (resourceName !== GetCurrentResourceName()) return;
    console.log('^3Hot Reload Server stopped^0');
});

RegisterCommand('hot::status', (source: number): void => {
    if (source !== 0) return;
    console.log(`^3Hot Reload Server status: ${WATCHER_API.isRunning() ? '^2RUNNING' : '^1STOPPED'}^0`);
}, false);