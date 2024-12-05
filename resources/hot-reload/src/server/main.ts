import WATCHER_API from "./class/app";

on('onResourceStart', (resourceName: string) => {
    if (resourceName === GetCurrentResourceName()) {
        console.log(`^3Hot Reload Server status: ${WATCHER_API.isRunning() ? '^2RUNNING' : '^1STOPPED'}^0`);
    };
});