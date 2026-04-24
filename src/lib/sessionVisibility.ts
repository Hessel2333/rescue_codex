import type { SessionMessage } from "../types/api";

const INTERNAL_PREFIXES = [
  "<environment_context>",
  "<environment_context",
  "<permissions instructions>",
  "<permissions instructions",
  "<app-context>",
  "<app-context",
  "<collaboration_mode>",
  "<collaboration_mode",
  "<skills_instructions>",
  "<skills_instructions",
  "<plugins_instructions>",
  "<plugins_instructions",
];

function normalizeText(text?: string | null) {
  return (text ?? "")
    .replace(/<image>[\s\S]*?<\/image>/gi, "")
    .trim()
    .replace(/\s+/g, " ");
}

function messageImageCount(message: SessionMessage) {
  return Math.max(message.imageUrls?.length ?? 0, message.image_urls?.length ?? 0);
}

function parseTimestamp(value?: string | null) {
  if (!value) {
    return null;
  }

  const parsed = Date.parse(value);
  return Number.isNaN(parsed) ? null : parsed;
}

export function isInternalMessageText(text?: string | null) {
  const normalized = (text ?? "").trimStart();
  return INTERNAL_PREFIXES.some((prefix) => normalized.startsWith(prefix));
}

export function isVisibleConversationMessage(message: SessionMessage) {
  if (message.role === "developer" || message.role === "system") {
    return false;
  }

  if (isInternalMessageText(message.text)) {
    return false;
  }

  return true;
}

export function dedupeVisibleMessages(messages: SessionMessage[]) {
  const seen = new Map<string, Array<{ timestamp: number; outputIndex: number }>>();
  const output: SessionMessage[] = [];

  for (const message of messages) {
    const isUserTextMessage = message.role === "user" && message.kind === "message";
    const key = [
      message.role ?? "",
      message.kind,
      isUserTextMessage ? "" : message.turnId ?? "",
      message.toolName ?? "",
      message.phase ?? "",
      normalizeText(message.text),
    ].join("|");
    const timestamp = parseTimestamp(message.ts);
    const matches = seen.get(key) ?? [];
    const duplicateSentinel = Number.MIN_SAFE_INTEGER;
    const compareTarget = timestamp ?? duplicateSentinel;
    const duplicate = matches.find((existing) => Math.abs(existing.timestamp - compareTarget) <= 2000);

    if (duplicate) {
      const existing = output[duplicate.outputIndex];
      const existingImageCount = messageImageCount(existing);
      const currentImageCount = messageImageCount(message);
      if (currentImageCount > existingImageCount) {
        output[duplicate.outputIndex] = message;
      }
      continue;
    }

    output.push(message);
    seen.set(key, [...matches, { timestamp: compareTarget, outputIndex: output.length - 1 }]);
  }

  return output;
}

export function sanitizeMessagePreview(text?: string | null) {
  if (!text || isInternalMessageText(text)) {
    return null;
  }

  return text;
}
