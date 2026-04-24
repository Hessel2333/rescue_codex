import { ComponentType, ReactNode, useMemo, useRef, useState } from "react";
import {
  ChevronDown,
  ChevronRight,
  Code2,
  Database,
  FileText,
  Globe2,
  Hash,
  ImageIcon,
  Search,
  Terminal,
  Wrench,
} from "lucide-react";
import clsx from "clsx";
import { Panel } from "../../components/Panel";
import { formatDateTime, formatDuration, formatOptionalText } from "../../lib/format";
import { dedupeVisibleMessages, isVisibleConversationMessage } from "../../lib/sessionVisibility";
import { SessionDetail, SessionMessage } from "../../types/api";

type SessionDetailPanelProps = {
  detail?: SessionDetail | null;
};

type ToolTone = "shell" | "search" | "file" | "image" | "data" | "code" | "generic";

type ToolConfig = {
  tone: ToolTone;
  label: string;
  Icon: ComponentType<{ className?: string }>;
};

type TurnNode =
  | { type: "message"; message: SessionMessage }
  | { type: "process"; id: string; messages: SessionMessage[] };

type ConversationTurn = {
  id: string;
  index: number;
  messages: SessionMessage[];
  nodes: TurnNode[];
  preview: string;
};

function roleOf(message: SessionMessage) {
  return (message.role ?? "").toLowerCase();
}

function kindOf(message: SessionMessage) {
  return message.kind.toLowerCase();
}

function isAssistantMessage(message: SessionMessage) {
  return roleOf(message) === "assistant" && kindOf(message) === "message";
}

function isUserMessage(message: SessionMessage) {
  return roleOf(message) === "user";
}

function isProcessMessage(message: SessionMessage) {
  const role = roleOf(message);
  const kind = kindOf(message);
  return Boolean(
    message.toolName ||
      role === "tool" ||
      kind.includes("tool") ||
      kind.includes("function") ||
      kind.includes("event") ||
      kind.includes("context"),
  );
}

function isToolDetailMessage(message: SessionMessage) {
  const role = roleOf(message);
  const kind = kindOf(message);
  return Boolean(
    message.toolName ||
      role === "tool" ||
      kind.includes("tool_call") ||
      kind.includes("tool_result") ||
      kind.includes("function"),
  );
}

function timestampMs(message: SessionMessage) {
  if (!message.ts) {
    return null;
  }
  const parsed = Date.parse(message.ts);
  return Number.isNaN(parsed) ? null : parsed;
}

function processDurationSec(messages: SessionMessage[]) {
  const timestamps = messages.map(timestampMs).filter((value): value is number => value !== null);
  if (timestamps.length < 2) {
    return null;
  }
  return Math.max(0, Math.round((Math.max(...timestamps) - Math.min(...timestamps)) / 1000));
}

function toolConfig(message: SessionMessage): ToolConfig {
  const raw = `${message.toolName ?? ""} ${message.kind} ${message.text ?? ""}`.toLowerCase();

  if (raw.includes("shell") || raw.includes("command") || raw.includes("powershell") || raw.includes("bash")) {
    return { tone: "shell", label: message.toolName || "Shell", Icon: Terminal };
  }
  if (raw.includes("search") || raw.includes("web")) {
    return { tone: "search", label: message.toolName || "Search", Icon: Search };
  }
  if (raw.includes("file") || raw.includes("fs") || raw.includes("read") || raw.includes("write")) {
    return { tone: "file", label: message.toolName || "File", Icon: FileText };
  }
  if (raw.includes("image") || raw.includes("screenshot")) {
    return { tone: "image", label: message.toolName || "Image", Icon: ImageIcon };
  }
  if (raw.includes("sqlite") || raw.includes("database") || raw.includes("query")) {
    return { tone: "data", label: message.toolName || "Data", Icon: Database };
  }
  if (raw.includes("http") || raw.includes("url") || raw.includes("browser")) {
    return { tone: "search", label: message.toolName || "Web", Icon: Globe2 };
  }
  if (raw.includes("patch") || raw.includes("code")) {
    return { tone: "code", label: message.toolName || "Code", Icon: Code2 };
  }

  return { tone: "generic", label: message.toolName || message.kind || "Tool", Icon: Wrench };
}

function messagePreview(message?: SessionMessage) {
  const raw = stripImagePlaceholders(message?.text) || message?.toolName || message?.kind || "无文本内容";
  const singleLine = raw.replace(/\s+/g, " ");
  return singleLine.length > 34 ? `${singleLine.slice(0, 34)}...` : singleLine;
}

function toolPreview(message: SessionMessage) {
  const raw = message.text?.trim() || message.toolName || message.kind;
  const singleLine = raw.replace(/\s+/g, " ");
  return singleLine.length > 120 ? `${singleLine.slice(0, 120)}...` : singleLine;
}

function normalizedMessageText(text?: string | null) {
  return stripImagePlaceholders(text).trim().replace(/\s+/g, " ");
}

function stripImagePlaceholders(text?: string | null) {
  return (text ?? "").replace(/<image>[\s\S]*?<\/image>/gi, "").trimEnd();
}

function messageImageUrls(message: SessionMessage) {
  return message.imageUrls?.length ? message.imageUrls : message.image_urls ?? [];
}

function codeBlockTone(language: string) {
  const normalized = language.toLowerCase();
  if (["sh", "bash", "zsh", "shell", "powershell", "ps1", "cmd"].includes(normalized)) {
    return "shell";
  }
  if (["json", "jsonl", "yaml", "yml", "toml"].includes(normalized)) {
    return "data";
  }
  if (["md", "markdown"].includes(normalized)) {
    return "doc";
  }
  return "code";
}

function renderInlineCode(text: string) {
  const parts = text.split(/(`[^`]+`)/g);
  return parts.map((part, index) => {
    if (part.startsWith("`") && part.endsWith("`")) {
      return (
        <code key={index} className="session-inline-code">
          {part.slice(1, -1)}
        </code>
      );
    }
    return <span key={index}>{part}</span>;
  });
}

function RichMessageText({ text }: { text?: string | null }) {
  const source = text?.trimEnd();
  if (!source) {
    return <p className="session-message__text">无文本内容</p>;
  }

  const parts: Array<{ type: "text"; value: string } | { type: "code"; language: string; value: string }> = [];
  const pattern = /```([^\n`]*)\n?([\s\S]*?)```/g;
  let cursor = 0;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(source)) !== null) {
    if (match.index > cursor) {
      parts.push({ type: "text", value: source.slice(cursor, match.index) });
    }
    parts.push({
      type: "code",
      language: match[1].trim() || "text",
      value: match[2].replace(/^\n|\n$/g, ""),
    });
    cursor = match.index + match[0].length;
  }

  if (cursor < source.length) {
    parts.push({ type: "text", value: source.slice(cursor) });
  }

  if (parts.length === 0) {
    parts.push({ type: "text", value: source });
  }

  return (
    <div className="session-rich-text">
      {parts.map((part, index) => {
        if (part.type === "code") {
          const tone = codeBlockTone(part.language);
          return (
            <figure key={index} className={clsx("session-code-block", `code-tone-${tone}`)}>
              <figcaption>{part.language}</figcaption>
              <pre>
                <code>{part.value}</code>
              </pre>
            </figure>
          );
        }

        return part.value
          .split(/\n{2,}/)
          .filter((paragraph) => paragraph.trim().length > 0)
          .map((paragraph, paragraphIndex) => {
            const lines = paragraph.split("\n");
            const isList = lines.every((line) => /^\s*(-|\*|\d+\.)\s+/.test(line.trim()));
            if (isList) {
              return (
                <ul key={`${index}-${paragraphIndex}`} className="session-message-list">
                  {lines.map((line, lineIndex) => (
                    <li key={lineIndex}>{renderInlineCode(line.replace(/^\s*(-|\*|\d+\.)\s+/, ""))}</li>
                  ))}
                </ul>
              );
            }

            return (
              <p key={`${index}-${paragraphIndex}`} className="session-message__text">
                {renderInlineCode(paragraph)}
              </p>
            );
          });
      })}
    </div>
  );
}

function flushProcess(nodes: TurnNode[], buffer: SessionMessage[], turnId: string) {
  if (buffer.length === 0) {
    return;
  }
  nodes.push({
    type: "process",
    id: `${turnId}-process-${nodes.length + 1}`,
    messages: [...buffer],
  });
  buffer.length = 0;
}

function buildTurnNodes(turnMessages: SessionMessage[], turnId: string) {
  const nodes: TurnNode[] = [];
  const processBuffer: SessionMessage[] = [];
  const seenProcessAssistantText = new Set<string>();
  const finalAssistantIndex = (() => {
    for (let index = turnMessages.length - 1; index >= 0; index -= 1) {
      if (isAssistantMessage(turnMessages[index])) {
        return index;
      }
    }
    return -1;
  })();
  const finalAssistantText =
    finalAssistantIndex >= 0 ? normalizedMessageText(turnMessages[finalAssistantIndex].text) : "";

  turnMessages.forEach((message, index) => {
    if (isUserMessage(message)) {
      flushProcess(nodes, processBuffer, turnId);
      nodes.push({ type: "message", message });
      return;
    }

    if (index === finalAssistantIndex) {
      flushProcess(nodes, processBuffer, turnId);
      nodes.push({ type: "message", message });
      return;
    }

    if (isAssistantMessage(message)) {
      const textKey = normalizedMessageText(message.text);
      if (textKey && textKey === finalAssistantText) {
        return;
      }
      if (textKey && seenProcessAssistantText.has(textKey)) {
        return;
      }
      seenProcessAssistantText.add(textKey);
    }

    if (isProcessMessage(message) || isAssistantMessage(message)) {
      processBuffer.push(message);
      return;
    }

    flushProcess(nodes, processBuffer, turnId);
    nodes.push({ type: "message", message });
  });

  flushProcess(nodes, processBuffer, turnId);
  return nodes;
}

function buildTurns(messages: SessionMessage[]) {
  const turns: Array<Omit<ConversationTurn, "nodes" | "preview">> = [];
  let current: Omit<ConversationTurn, "nodes" | "preview"> | null = null;

  for (const message of messages) {
    const startsTurn = isUserMessage(message) || !current;

    if (startsTurn) {
      current = {
        id: message.turnId || message.id || `turn-${turns.length + 1}`,
        index: turns.length + 1,
        messages: [],
      };
      turns.push(current);
    }

    current!.messages.push(message);
  }

  return turns.map((turn) => ({
    ...turn,
    nodes: buildTurnNodes(turn.messages, turn.id),
    preview: messagePreview(turn.messages.find(isUserMessage) ?? turn.messages[0]),
  }));
}

function MessageBubble({ message }: { message: SessionMessage }) {
  const role = roleOf(message) || "unknown";
  const isUser = role === "user";
  const isAssistant = role === "assistant";
  const imageUrls = messageImageUrls(message);
  const text = stripImagePlaceholders(message.text);

  return (
    <article className={clsx("session-message", isUser && "is-user", isAssistant && "is-assistant")}>
      <div className="session-message__meta">
        <span className="metric-pill">{formatOptionalText(message.role, "unknown")}</span>
        <span>{message.kind}</span>
        <span>{formatDateTime(message.ts)}</span>
      </div>
      {imageUrls.length > 0 ? <MessageImages urls={imageUrls} /> : null}
      {text.trim() || imageUrls.length === 0 ? <RichMessageText text={text} /> : null}
    </article>
  );
}

function MessageImages({ urls }: { urls: string[] }) {
  const [activeUrl, setActiveUrl] = useState<string | null>(null);

  return (
    <>
      <div className="session-message-images">
        {urls.map((url, index) => (
          <button
            key={`${url.slice(0, 80)}-${index}`}
            type="button"
            className="session-message-image"
            onClick={() => setActiveUrl(url)}
            aria-label={`放大查看图片 ${index + 1}`}
          >
            <img src={url} alt={`会话图片 ${index + 1}`} loading="lazy" />
          </button>
        ))}
      </div>

      {activeUrl ? (
        <div className="session-image-lightbox" role="dialog" aria-modal="true" onClick={() => setActiveUrl(null)}>
          <button type="button" className="session-image-lightbox__close" onClick={() => setActiveUrl(null)}>
            关闭
          </button>
          <img src={activeUrl} alt="放大的会话图片" onClick={(event) => event.stopPropagation()} />
        </div>
      ) : null}
    </>
  );
}

function ProcessTextMessage({ message }: { message: SessionMessage }) {
  return (
    <div className="session-process-text">
      <RichMessageText text={message.text} />
    </div>
  );
}

function ProcessMessage({ message }: { message: SessionMessage }) {
  const [expanded, setExpanded] = useState(false);
  const config = toolConfig(message);
  const preview = toolPreview(message);

  return (
    <div className={clsx("session-process-message", `tool-tone-${config.tone}`)}>
      <button type="button" className="session-process-message__header" onClick={() => setExpanded((value) => !value)}>
        {expanded ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
        <span className="session-tool-badge">
          <config.Icon className="h-3.5 w-3.5" />
          {config.label}
        </span>
        <span className="session-process-message__kind">{message.kind}</span>
        <span className="session-process-message__preview">{preview}</span>
      </button>
      {expanded && message.text ? <pre className="session-process-message__text">{message.text}</pre> : null}
    </div>
  );
}

function ProcessGroup({ messages }: { messages: SessionMessage[] }) {
  const [expanded, setExpanded] = useState(false);
  const duration = processDurationSec(messages);
  const toolCount = messages.filter(isToolDetailMessage).length;
  const label = duration === null ? "已处理" : `已处理 ${formatDuration(duration)}`;

  return (
    <div className={clsx("session-process-group", expanded && "is-expanded")}>
      <button type="button" className="session-process-toggle" onClick={() => setExpanded((value) => !value)}>
        {expanded ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
        <span>{label}</span>
        {toolCount > 0 ? <span className="session-process-toggle__count">{toolCount} 条工具</span> : null}
      </button>

      {expanded ? (
        <div className="session-process-body">
          {messages.map((message) =>
            isToolDetailMessage(message) ? (
              <ProcessMessage key={message.id} message={message} />
            ) : (
              <ProcessTextMessage key={message.id} message={message} />
            ),
          )}
        </div>
      ) : null}
    </div>
  );
}

function TurnBlock({
  turn,
  register,
}: {
  turn: ConversationTurn;
  register: (id: string, element: HTMLElement | null) => void;
}) {
  return (
    <section ref={(element) => register(turn.id, element)} className="session-turn">
      <div className="session-turn__header">
        <div className="session-turn__badge">
          <Hash className="h-3.5 w-3.5" />
          <span>第 {turn.index} 轮</span>
        </div>
      </div>

      <div className="session-turn__messages">
        {turn.nodes.map((node): ReactNode => {
          if (node.type === "message") {
            return <MessageBubble key={node.message.id} message={node.message} />;
          }
          return <ProcessGroup key={node.id} messages={node.messages} />;
        })}
      </div>
    </section>
  );
}

export function SessionDetailPanel({ detail }: SessionDetailPanelProps) {
  const turnRefs = useRef(new Map<string, HTMLElement>());

  const turns = useMemo(() => {
    if (!detail) {
      return [];
    }
    const messages = dedupeVisibleMessages(detail.messages.filter(isVisibleConversationMessage));
    return buildTurns(messages);
  }, [detail]);

  function registerTurn(id: string, element: HTMLElement | null) {
    if (element) {
      turnRefs.current.set(id, element);
    } else {
      turnRefs.current.delete(id);
    }
  }

  function jumpToTurn(id: string) {
    turnRefs.current.get(id)?.scrollIntoView({ block: "start", behavior: "smooth" });
  }

  if (!detail) {
    return (
      <Panel title="会话详情" className="session-detail-panel">
        <div className="empty-state">当前还没有选中的会话。</div>
      </Panel>
    );
  }

  const { session } = detail;

  return (
    <Panel title={formatOptionalText(session.threadTitle, "未命名会话")} className="session-detail-panel">
      <div className="session-detail">
        <aside className="session-turn-nav" aria-label="对话轮次">
          {turns.map((turn) => (
            <button key={turn.id} type="button" className="session-turn-nav__item" onClick={() => jumpToTurn(turn.id)}>
              <span className="session-turn-nav__index">{turn.index}</span>
              <span className="session-turn-nav__preview">{turn.preview}</span>
            </button>
          ))}
        </aside>

        <div className="session-detail__content">
          <div className="session-detail__meta-bar">
            <div className="session-detail__meta-item is-wide">
              <span className="meta-label">工作目录</span>
              <span className="mono-value">{formatOptionalText(session.cwd)}</span>
            </div>
            <div className="session-detail__meta-item">
              <span className="meta-label">更新时间</span>
              <span>{formatDateTime(session.updatedAt)}</span>
            </div>
            <div className="session-detail__meta-item">
              <span className="meta-label">时长</span>
              <span>{formatDuration(session.durationSec)}</span>
            </div>
            <div className="session-detail__meta-item">
              <span className="meta-label">工具</span>
              <span>{session.toolCallCount}</span>
            </div>
          </div>

          <div className="session-turn-list">
            {turns.length === 0 ? <div className="empty-state">当前会话没有可展示的归一化消息。</div> : null}
            {turns.map((turn) => (
              <TurnBlock key={turn.id} turn={turn} register={registerTurn} />
            ))}
          </div>
        </div>
      </div>
    </Panel>
  );
}
