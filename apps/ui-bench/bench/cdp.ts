// Minimal Chrome DevTools Protocol client over Bun's built-in WebSocket (flat sessions).
type Pending = { resolve: (v: any) => void; reject: (e: Error) => void; method: string };
type Listener = (params: any, sessionId?: string) => void;

export class CDP {
  #ws: WebSocket;
  #id = 0;
  #pending = new Map<number, Pending>();
  #listeners = new Map<string, Set<Listener>>();

  private constructor(ws: WebSocket) {
    this.#ws = ws;
    ws.addEventListener("message", ev => {
      const msg = JSON.parse(String(ev.data));
      if (msg.id !== undefined) {
        const p = this.#pending.get(msg.id);
        if (!p) return;
        this.#pending.delete(msg.id);
        if (msg.error) p.reject(new Error(`${p.method}: ${msg.error.message}`));
        else p.resolve(msg.result);
      } else if (msg.method) {
        for (const l of this.#listeners.get(msg.method) ?? []) l(msg.params, msg.sessionId);
      }
    });
  }

  static async connect(url: string): Promise<CDP> {
    const ws = new WebSocket(url);
    await new Promise<void>((resolve, reject) => {
      ws.addEventListener("open", () => resolve(), { once: true });
      ws.addEventListener("error", () => reject(new Error(`cannot open ${url}`)), { once: true });
    });
    return new CDP(ws);
  }

  send<T = any>(method: string, params: Record<string, unknown> = {}, sessionId?: string): Promise<T> {
    const id = ++this.#id;
    const payload: Record<string, unknown> = { id, method, params };
    if (sessionId) payload.sessionId = sessionId;
    return new Promise((resolve, reject) => {
      this.#pending.set(id, { resolve, reject, method });
      this.#ws.send(JSON.stringify(payload));
    });
  }

  on(method: string, listener: Listener): () => void {
    let set = this.#listeners.get(method);
    if (!set) this.#listeners.set(method, (set = new Set()));
    set.add(listener);
    return () => set!.delete(listener);
  }

  close() {
    this.#ws.close();
  }
}
