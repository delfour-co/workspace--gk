import { useState, useEffect, useCallback, useRef } from 'react';
import type { Message, ServerMessage, ConnectionStatus } from '../types';
import { parseToolResult } from '../types';

const WS_URL = 'ws://localhost:8888/ws';

export function useWebSocket() {
  const [status, setStatus] = useState<ConnectionStatus>('connecting');
  const [messages, setMessages] = useState<Message[]>([]);
  const [isProcessing, setIsProcessing] = useState(false);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [userEmail, setUserEmail] = useState<string>('');
  const wsRef = useRef<WebSocket | null>(null);
  const currentToolRef = useRef<string>('');
  const currentToolIdRef = useRef<string>('');
  const hasToolDataRef = useRef<boolean>(false);

  useEffect(() => {
    const ws = new WebSocket(WS_URL);
    wsRef.current = ws;

    ws.onopen = () => {
      setStatus('connected');
    };

    ws.onmessage = (event) => {
      try {
        const data: ServerMessage = JSON.parse(event.data);
        console.log('Received:', data);

        switch (data.type) {
          case 'auth_success':
            setIsAuthenticated(true);
            if (data.email) {
              setUserEmail(data.email);
              addMessage({
                id: Date.now().toString(),
                type: 'assistant',
                content: `Bonjour ! Je suis votre assistant email. Comment puis-je vous aider ?`,
                timestamp: new Date(),
              });
            }
            break;

          case 'chunk':
            setIsProcessing(false);
            if (data.content) {
              addMessage({
                id: Date.now().toString(),
                type: 'assistant',
                content: data.content,
                timestamp: new Date(),
              });
            }
            break;

          case 'tool_call':
            if (data.tool) {
              currentToolRef.current = data.tool;
              const toolId = `tool-${Date.now()}`;
              currentToolIdRef.current = toolId;
              hasToolDataRef.current = false;
              addMessage({
                id: toolId,
                type: 'tool',
                content: `Appel: ${formatToolName(data.tool)}...`,
                timestamp: new Date(),
              });
            }
            break;

          case 'tool_result':
            if (data.result !== undefined) {
              const toolName = currentToolRef.current || 'unknown';
              const toolData = parseToolResult(toolName, data.result);
              const toolId = currentToolIdRef.current;

              // Track if we have structured data to display
              hasToolDataRef.current = !!(
                (toolData.emails && toolData.emails.length > 0) ||
                toolData.email ||
                toolData.success
              );

              // Replace the "Appel:..." message with the result
              updateMessage(toolId, {
                id: toolId,
                type: 'tool',
                content: formatToolResultSummary(toolName, data.result),
                timestamp: new Date(),
                toolData,
              });
            }
            break;

          case 'done':
            setIsProcessing(false);
            // Don't show LLM text response if we already displayed structured data
            if (data.content && !hasToolDataRef.current) {
              addMessage({
                id: Date.now().toString(),
                type: 'assistant',
                content: data.content,
                timestamp: new Date(),
              });
            }
            // Reset for next interaction
            hasToolDataRef.current = false;
            break;

          case 'error':
            setIsProcessing(false);
            if (data.message) {
              addMessage({
                id: Date.now().toString(),
                type: 'error',
                content: data.message,
                timestamp: new Date(),
              });
            }
            break;
        }
      } catch (e) {
        console.error('Parse error:', e);
        setIsProcessing(false);
        addMessage({
          id: Date.now().toString(),
          type: 'error',
          content: `Erreur de parsing: ${(e as Error).message}`,
          timestamp: new Date(),
        });
      }
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      setStatus('disconnected');
      addMessage({
        id: Date.now().toString(),
        type: 'error',
        content: 'Erreur de connexion WebSocket',
        timestamp: new Date(),
      });
    };

    ws.onclose = () => {
      setStatus('disconnected');
    };

    return () => {
      ws.close();
    };
  }, []);

  const addMessage = useCallback((message: Message) => {
    setMessages((prev) => [...prev, message]);
  }, []);

  const updateMessage = useCallback((id: string, message: Message) => {
    setMessages((prev) => prev.map((m) => (m.id === id ? message : m)));
  }, []);

  const clearMessages = useCallback(() => {
    setMessages([]);
  }, []);

  const authenticate = useCallback((email: string) => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
      return;
    }

    const payload = {
      type: 'auth',
      email,
    };

    wsRef.current.send(JSON.stringify(payload));
  }, []);

  const sendMessage = useCallback((text: string) => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
      return;
    }

    // Add user message to UI
    addMessage({
      id: Date.now().toString(),
      type: 'user',
      content: text,
      timestamp: new Date(),
    });

    // Set processing state
    setIsProcessing(true);

    // Send to server
    const payload = {
      type: 'chat',
      message: text,
    };

    wsRef.current.send(JSON.stringify(payload));
  }, [addMessage]);

  const logout = useCallback(() => {
    setIsAuthenticated(false);
    setUserEmail('');
    setMessages([]);
    // Reconnect WebSocket
    if (wsRef.current) {
      wsRef.current.close();
    }
  }, []);

  return {
    status,
    messages,
    sendMessage,
    authenticate,
    isAuthenticated,
    userEmail,
    isProcessing,
    clearMessages,
    logout,
  };
}

// Format tool name for display
function formatToolName(tool: string): string {
  const names: Record<string, string> = {
    list_emails: 'Récupération des emails',
    read_email: 'Lecture de l\'email',
    send_email: 'Envoi de l\'email',
    search_emails: 'Recherche d\'emails',
  };
  return names[tool] || tool;
}

// Format tool result summary
function formatToolResultSummary(tool: string, result: unknown): string {
  if (!result) return 'Aucun résultat';

  if (tool === 'list_emails' || tool === 'search_emails') {
    if (Array.isArray(result)) {
      return `${result.length} email(s) trouvé(s)`;
    }
    const obj = result as Record<string, unknown>;
    if (Array.isArray(obj.emails)) {
      return `${obj.emails.length} email(s) trouvé(s)`;
    }
  }

  if (tool === 'read_email') {
    return 'Email chargé';
  }

  if (tool === 'send_email') {
    const obj = result as Record<string, unknown>;
    if (obj.success || obj.status === 'sent') {
      return 'Email envoyé avec succès';
    }
    if (obj.error) {
      return `Erreur: ${obj.error}`;
    }
  }

  return 'Opération terminée';
}
