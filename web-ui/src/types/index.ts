// Message types for UI
export interface Message {
  id: string;
  type: 'user' | 'assistant' | 'tool' | 'error';
  content: string;
  timestamp: Date;
  // Optional structured data for tool results
  toolData?: ToolResultData;
}

// Email structure from MCP tools
export interface Email {
  id: string;
  from: string;
  to: string;
  subject: string;
  date: string;
  preview?: string;
  body?: string;
  flags?: string[];
  size?: number;
}

// Tool result data for rich rendering
export interface ToolResultData {
  tool: string;
  emails?: Email[];
  email?: Email;
  success?: boolean;
  error?: string;
  count?: number;
}

// Server message from WebSocket
export interface ServerMessage {
  type: 'auth_success' | 'chunk' | 'tool_call' | 'tool_result' | 'done' | 'error';
  email?: string;
  content?: string;
  tool?: string;
  arguments?: Record<string, unknown>;
  result?: unknown;
  message?: string;
}

// Client message to WebSocket
export interface ClientMessage {
  type: 'auth' | 'chat';
  email?: string;
  message?: string;
}

// Connection state
export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected';

// Parse tool result into structured data
export function parseToolResult(tool: string, result: unknown): ToolResultData {
  const data: ToolResultData = { tool };

  if (!result) {
    return data;
  }

  try {
    // Handle list_emails result
    if (tool === 'list_emails' || tool === 'search_emails') {
      if (Array.isArray(result)) {
        data.emails = result.map(parseEmail);
        data.count = result.length;
      } else if (typeof result === 'object' && result !== null) {
        const obj = result as Record<string, unknown>;
        if (Array.isArray(obj.emails)) {
          data.emails = obj.emails.map(parseEmail);
          data.count = obj.emails.length;
        }
      }
    }

    // Handle read_email result
    if (tool === 'read_email') {
      if (typeof result === 'object' && result !== null) {
        data.email = parseEmail(result);
      }
    }

    // Handle send_email result
    if (tool === 'send_email') {
      if (typeof result === 'object' && result !== null) {
        const obj = result as Record<string, unknown>;
        data.success = obj.success === true || obj.status === 'sent';
        if (obj.error) {
          data.error = String(obj.error);
        }
      }
    }
  } catch {
    // Return basic data if parsing fails
  }

  return data;
}

// Parse a raw object into Email structure
function parseEmail(raw: unknown): Email {
  if (typeof raw !== 'object' || raw === null) {
    return {
      id: '',
      from: 'Unknown',
      to: '',
      subject: '(No subject)',
      date: '',
    };
  }

  const obj = raw as Record<string, unknown>;

  return {
    id: String(obj.id || obj.message_id || ''),
    from: String(obj.from || obj.sender || 'Unknown'),
    to: String(obj.to || obj.recipient || ''),
    subject: String(obj.subject || '(No subject)'),
    date: String(obj.date || obj.received || ''),
    preview: obj.preview ? String(obj.preview) : undefined,
    body: obj.body ? String(obj.body) : undefined,
    flags: Array.isArray(obj.flags) ? obj.flags.map(String) : undefined,
    size: typeof obj.size === 'number' ? obj.size : undefined,
  };
}
