declare module "ws" {
  type WebSocketData = string | ArrayBuffer | Buffer | Buffer[];

  type WebSocketMessageListener = (
    data: WebSocketData,
    isBinary: boolean,
  ) => void;

  type WebSocketCloseListener = (code: number, reason: Buffer) => void;

  type WebSocketOptions = {
    headers?: Record<string, string>;
    maxPayload?: number;
    perMessageDeflate?: boolean;
  };

  class WebSocket {
    static readonly CONNECTING: number;
    static readonly OPEN: number;
    static readonly CLOSING: number;
    static readonly CLOSED: number;

    readonly readyState: number;

    constructor(address: string | URL, options?: WebSocketOptions);

    on(event: "open", listener: () => void): this;
    on(event: "message", listener: WebSocketMessageListener): this;
    on(event: "error", listener: (error: Error) => void): this;
    on(event: "close", listener: WebSocketCloseListener): this;
    once(event: "open", listener: () => void): this;
    once(event: "error", listener: (error: Error) => void): this;
    once(event: "close", listener: WebSocketCloseListener): this;
    removeListener(event: string, listener: (...args: any[]) => void): this;

    send(data: string, callback?: (error?: Error) => void): void;
    close(code?: number, data?: string): void;
    terminate(): void;
  }

  export default WebSocket;
}
