import { WebSocketServer, WebSocket } from 'ws';

interface ResourceChange {
  resource_name: string;
  change_type: 'FileModified' | 'FileAdded' | 'FileRemoved' | 'ManifestChanged';
}

export class HotReloadServer {
  private wss: WebSocketServer;
  private clients: Set<WebSocket> = new Set();
  // @ts-ignore
  private locale: string;

  // @ts-ignore
  constructor(private port: number) {
    this.wss = new WebSocketServer({ port, host: 'localhost' });
    this.locale = GetConvar('locale', 'en-US');

    this.setupWebSocket();
    console.log(`^2Hot Reload WebSocket started on port ${port}^0`);
  }
  
  public isRunning(): boolean {
    return this.wss?.address() !== null;
  }

  private setupWebSocket(): void {
    this.wss.on('connection', (ws: WebSocket): void => {
      console.log('^2New connection to Hot Reload^0');
      this.clients.add(ws);

      ws.on('message', async (data: any): Promise<void> => {
        try {
          const change: ResourceChange = JSON.parse(data.toString());
          await this.handleResourceChange(change);
        } catch (error) {
          console.error('^1Error processing message:', error, '^0');
        }
      });

      ws.on('close', (): void => {
        this.clients.delete(ws);
        console.log('^3Client Hot Reload disconnected^0');
      });
    });
  }

  private sendMessageToWatcher(message: string): void {
    // const date = new Date().toLocaleString(this.locale, { hour12: false });

    this.wss.clients.forEach((client: WebSocket): void => {
      if (client.readyState === WebSocket.OPEN) {
        // client.send(`[${date}] - ${message}`);
        client.send(message);
      }
    });
  }

  private async handleResourceChange(change: ResourceChange): Promise<void> {
    const { resource_name, change_type } = change;

    //console.log(`^3Changement détecté dans ${resource_name}: ${change_type} - ${file_path}^0`);

    if (GetResourceState(resource_name) === 'missing') {
      console.log(`^1Ressource ${resource_name} introuvable^0`);
      return;
    }

    try {
      switch (change_type) {
        case 'ManifestChanged':
        case 'FileAdded':
        case 'FileRemoved':
          ExecuteCommand('refresh');
          this.sendMessageToWatcher(`refresh`);
          await this.wait(500);
          ExecuteCommand(`ensure ${resource_name}`);
          this.sendMessageToWatcher(`ensure ${resource_name}`);
          break;

        case 'FileModified':
          ExecuteCommand(`ensure ${resource_name}`);
          this.sendMessageToWatcher(`ensure ${resource_name}`);
          break;
      }
    } catch (error) {
      console.error(`^1Error restarting ${resource_name}:`, error, '^0');
    }
  }

  private wait(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

// set hot::ws::port integer (in server.cfg)
const WATCHER_API = new HotReloadServer(GetConvarInt('hot::ws::port', 3091)); 
export default WATCHER_API;