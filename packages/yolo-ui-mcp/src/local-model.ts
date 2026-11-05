export interface LocalModelRequest {
  prompt: string;
  task?: "design" | "codegen" | "review";
  model?: string;
  baseUrl?: string;
  maxTokens?: number;
}

const TASK_SYSTEM: Record<string, string> = {
  design:
    "You are the Yolo UI design assistant. Propose concise, accessible UI designs (WCAG 2.2 AA) using Yolo theme presets and design tokens; no emoji.",
  codegen:
    "You are the Yolo UI code generator. Output typed TSX or Dioxus 0.8 RSX only, themed through yolo-tokens presets, no raw HTML/CSS/JS files, no emoji.",
  review:
    "You are the Yolo UI reviewer. List concrete accessibility, layout and token issues with fixes; no emoji.",
};

const LOOPBACK = /^https?:\/\/(127\.0\.0\.1|localhost|\[::1\])(:\d+)?(\/|$)/i;

/** Sends one chat turn to the yolo-ai local llama.cpp server (OpenAI-compatible, loopback only). */
export async function runLocalModel(req: LocalModelRequest): Promise<{
  model: string;
  baseUrl: string;
  text: string;
  usage?: unknown;
}> {
  const baseUrl = (
    req.baseUrl ??
    process.env.YOLO_LOCAL_LLM_BASE_URL ??
    "http://127.0.0.1:8083/v1"
  ).replace(/\/+$/, "");
  if (!LOOPBACK.test(baseUrl)) {
    throw new Error(`ui_local_model only talks to loopback endpoints, got ${baseUrl}`);
  }
  const model = req.model ?? process.env.YOLO_LOCAL_LLM_MODEL ?? "local-qwen-small";
  const res = await fetch(`${baseUrl}/chat/completions`, {
    method: "POST",
    headers: { "content-type": "application/json", authorization: "Bearer local" },
    body: JSON.stringify({
      model,
      max_tokens: req.maxTokens ?? 1024,
      temperature: 0.4,
      messages: [
        { role: "system", content: TASK_SYSTEM[req.task ?? "design"] },
        { role: "user", content: req.prompt },
      ],
    }),
    signal: AbortSignal.timeout(180_000),
  });
  if (!res.ok) {
    throw new Error(`local model ${res.status}: ${(await res.text()).slice(0, 300)}`);
  }
  const json = (await res.json()) as {
    choices: { message: { content: string } }[];
    usage?: unknown;
  };
  return { model, baseUrl, text: json.choices[0]?.message.content ?? "", usage: json.usage };
}
