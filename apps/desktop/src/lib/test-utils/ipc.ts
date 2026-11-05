export type IpcHandler = (args?: Record<string, unknown>) => unknown;

export type IpcCall = {
  command: string;
  args?: Record<string, unknown>;
};

export class IpcMock {
  private handlers = new Map<string, IpcHandler>();
  public calls: IpcCall[] = [];

  handle(command: string, handler: IpcHandler): this {
    this.handlers.set(command, handler);
    return this;
  }

  async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    this.calls.push({ command, args });
    const handler = this.handlers.get(command);
    if (!handler) {
      throw new Error(`No mock handler for ${command}`);
    }
    return (await handler(args)) as T;
  }

  reset(): void {
    this.handlers.clear();
    this.calls = [];
  }
}
