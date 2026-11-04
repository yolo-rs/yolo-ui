#!/usr/bin/env bun
import { executeUiTool, MCP_TOOL_DEFINITIONS } from "./tools";

export interface JsonRpcRequest {
  jsonrpc: "2.0";
  id?: string | number | null;
  method: string;
  params?: Record<string, unknown>;
}

export interface JsonRpcResponse {
  jsonrpc: "2.0";
  id: string | number | null;
  result?: unknown;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

export async function handleMcpJsonRpcRequest(
  req: JsonRpcRequest,
): Promise<JsonRpcResponse | null> {
  const id = req.id ?? null;

  if (
    req.method === "notifications/initialized" ||
    req.method.startsWith("notifications/")
  ) {
    return null;
  }

  switch (req.method) {
    case "initialize":
      return {
        jsonrpc: "2.0",
        id,
        result: {
          protocolVersion: "2024-11-05",
          capabilities: {
            tools: { listChanged: false },
          },
          serverInfo: {
            name: "@yolo-rs/ui-mcp",
            version: "0.8.0",
          },
        },
      };

    case "ping":
      return {
        jsonrpc: "2.0",
        id,
        result: {},
      };

    case "tools/list":
      return {
        jsonrpc: "2.0",
        id,
        result: {
          tools: MCP_TOOL_DEFINITIONS,
        },
      };

    case "tools/call": {
      const toolName = String(req.params?.name ?? "");
      const toolArgs =
        (req.params?.arguments as Record<string, unknown> | undefined) ?? {};
      try {
        const payload = await executeUiTool(toolName, toolArgs);
        return {
          jsonrpc: "2.0",
          id,
          result: {
            content: [
              {
                type: "text",
                text:
                  typeof payload === "string"
                    ? payload
                    : JSON.stringify(payload, null, 2),
              },
            ],
            isError: false,
          },
        };
      } catch (err) {
        return {
          jsonrpc: "2.0",
          id,
          result: {
            content: [
              {
                type: "text",
                text: err instanceof Error ? err.message : String(err),
              },
            ],
            isError: true,
          },
        };
      }
    }

    default:
      return {
        jsonrpc: "2.0",
        id,
        error: {
          code: -32601,
          message: `Method not found: ${req.method}`,
        },
      };
  }
}

export async function startMcpStdioServer(): Promise<void> {
  const decoder = new TextDecoder();
  let buffer = "";

  for await (const chunk of Bun.stdin.stream()) {
    buffer += decoder.decode(chunk, { stream: true });
    let newlineIdx = buffer.indexOf("\n");
    while (newlineIdx !== -1) {
      const line = buffer.slice(0, newlineIdx).trim();
      buffer = buffer.slice(newlineIdx + 1);
      if (line.length > 0 && line.startsWith("{")) {
        try {
          const req = JSON.parse(line) as JsonRpcRequest;
          const resp = await handleMcpJsonRpcRequest(req);
          if (resp) {
            process.stdout.write(`${JSON.stringify(resp)}\n`);
          }
        } catch (err) {
          const errResp: JsonRpcResponse = {
            jsonrpc: "2.0",
            id: null,
            error: {
              code: -32700,
              message: `Parse error: ${err instanceof Error ? err.message : String(err)}`,
            },
          };
          process.stdout.write(`${JSON.stringify(errResp)}\n`);
        }
      }
      newlineIdx = buffer.indexOf("\n");
    }
  }
}

if (import.meta.main) {
  await startMcpStdioServer();
}
