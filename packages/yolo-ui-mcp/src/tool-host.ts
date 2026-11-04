#!/usr/bin/env bun
import { executeUiTool, MCP_TOOL_DEFINITIONS } from "./tools";

export async function runToolHostCli(
  argv: string[] = process.argv.slice(2),
): Promise<number> {
  const [command = "list", toolName, rawJson] = argv;

  if (command === "list") {
    console.log(JSON.stringify({ tools: MCP_TOOL_DEFINITIONS }, null, 2));
    return 0;
  }

  if (command === "call") {
    if (!toolName) {
      console.error(
        "Usage: bun packages/yolo-ui-mcp/src/tool-host.ts call <tool_name> '<json_args>'",
      );
      return 1;
    }
    let parsedArgs: Record<string, unknown> = {};
    if (rawJson && rawJson.trim().length > 0) {
      try {
        parsedArgs = JSON.parse(rawJson) as Record<string, unknown>;
      } catch (err) {
        console.error(
          `Invalid JSON arguments: ${err instanceof Error ? err.message : String(err)}`,
        );
        return 1;
      }
    }
    try {
      const output = await executeUiTool(toolName, parsedArgs);
      console.log(JSON.stringify(output, null, 2));
      return 0;
    } catch (err) {
      console.error(
        JSON.stringify({
          error: err instanceof Error ? err.message : String(err),
        }),
      );
      return 1;
    }
  }

  console.error(
    `Unknown tool-host command '${command}'. Expected 'list' or 'call'.`,
  );
  return 1;
}

if (import.meta.main) {
  process.exit(await runToolHostCli());
}
