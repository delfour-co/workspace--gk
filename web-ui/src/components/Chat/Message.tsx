import type { Message as MessageType } from '../../types';
import { EmailCard, EmailList } from './EmailCard';

interface MessageProps {
  message: MessageType;
}

export function Message({ message }: MessageProps) {
  const { type, content, toolData, timestamp } = message;

  // Format timestamp
  const formatTime = (date: Date) => {
    return date.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' });
  };

  // Tool call message (when AI is calling a tool)
  if (type === 'tool' && content.startsWith('Appel:')) {
    return (
      <div className="flex items-center gap-2 text-xs text-slate-400">
        <svg className="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24">
          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
          <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
        </svg>
        <span className="italic">{content}</span>
      </div>
    );
  }

  // Tool result with structured email data
  if (type === 'tool' && toolData) {
    return (
      <div className="space-y-2">
        {/* List of emails */}
        {toolData.emails && toolData.emails.length > 0 && (
          <EmailList emails={toolData.emails} title="Emails trouvés" />
        )}

        {/* Single email detail */}
        {toolData.email && (
          <EmailCard email={toolData.email} expanded={true} />
        )}

        {/* Send email success */}
        {toolData.tool === 'send_email' && toolData.success && (
          <div className="flex items-center gap-2 p-3 bg-green-50 border border-green-200 rounded-lg text-green-700 text-sm">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
            </svg>
            Email envoyé avec succès
          </div>
        )}

        {/* Error */}
        {toolData.error && (
          <div className="flex items-center gap-2 p-3 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm">
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
            {toolData.error}
          </div>
        )}

        {/* Empty result */}
        {!toolData.emails && !toolData.email && !toolData.success && !toolData.error && (
          <div className="text-xs text-slate-400 italic">
            {content}
          </div>
        )}
      </div>
    );
  }

  // Generic tool/system message
  if (type === 'tool') {
    return (
      <div className="text-xs text-slate-400 italic pl-2 border-l-2 border-slate-200">
        {content}
      </div>
    );
  }

  // Error message
  if (type === 'error') {
    return (
      <div className="flex items-start gap-2 p-3 bg-red-50 border border-red-200 rounded-lg">
        <svg className="w-4 h-4 text-red-500 mt-0.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span className="text-red-700 text-sm">{content}</span>
      </div>
    );
  }

  // User message - aligned right with gray background
  if (type === 'user') {
    return (
      <div className="flex justify-end">
        <div className="bg-slate-100 rounded-2xl px-4 py-2.5 max-w-[80%]">
          <div className="text-slate-800 whitespace-pre-wrap">
            {content}
          </div>
          <div className="text-[10px] text-slate-400 mt-1 text-right">
            {formatTime(timestamp)}
          </div>
        </div>
      </div>
    );
  }

  // Assistant message - aligned left, no background
  return (
    <div className="flex justify-start">
      <div className="max-w-[85%]">
        <div className="text-slate-800 whitespace-pre-wrap leading-relaxed">
          {renderMarkdown(content)}
        </div>
      </div>
    </div>
  );
}

// Simple markdown-like rendering
function renderMarkdown(text: string): React.ReactNode {
  if (!text) return null;

  // Split into lines and process
  const lines = text.split('\n');
  const elements: React.ReactNode[] = [];

  let inCodeBlock = false;
  let codeContent: string[] = [];

  lines.forEach((line, index) => {
    // Code block start/end
    if (line.startsWith('```')) {
      if (!inCodeBlock) {
        inCodeBlock = true;
        codeContent = [];
      } else {
        // End of code block
        elements.push(
          <pre key={index} className="bg-slate-100 rounded-lg p-3 my-2 overflow-x-auto text-sm font-mono">
            <code>{codeContent.join('\n')}</code>
          </pre>
        );
        inCodeBlock = false;
        codeContent = [];
      }
      return;
    }

    if (inCodeBlock) {
      codeContent.push(line);
      return;
    }

    // Headers
    if (line.startsWith('### ')) {
      elements.push(
        <h3 key={index} className="font-semibold text-slate-900 mt-3 mb-1">
          {line.slice(4)}
        </h3>
      );
      return;
    }
    if (line.startsWith('## ')) {
      elements.push(
        <h2 key={index} className="font-bold text-slate-900 mt-4 mb-2 text-lg">
          {line.slice(3)}
        </h2>
      );
      return;
    }
    if (line.startsWith('# ')) {
      elements.push(
        <h1 key={index} className="font-bold text-slate-900 mt-4 mb-2 text-xl">
          {line.slice(2)}
        </h1>
      );
      return;
    }

    // Bullet points
    if (line.startsWith('- ') || line.startsWith('* ')) {
      elements.push(
        <div key={index} className="flex gap-2 pl-2">
          <span className="text-slate-400">•</span>
          <span>{renderInlineMarkdown(line.slice(2))}</span>
        </div>
      );
      return;
    }

    // Numbered lists
    const numberedMatch = line.match(/^(\d+)\.\s/);
    if (numberedMatch) {
      elements.push(
        <div key={index} className="flex gap-2 pl-2">
          <span className="text-slate-500 w-4">{numberedMatch[1]}.</span>
          <span>{renderInlineMarkdown(line.slice(numberedMatch[0].length))}</span>
        </div>
      );
      return;
    }

    // Empty line
    if (line.trim() === '') {
      elements.push(<div key={index} className="h-2" />);
      return;
    }

    // Regular paragraph
    elements.push(
      <p key={index} className="my-1">
        {renderInlineMarkdown(line)}
      </p>
    );
  });

  return <>{elements}</>;
}

// Render inline markdown (bold, italic, code)
function renderInlineMarkdown(text: string): React.ReactNode {
  if (!text) return null;

  // Process inline code first
  const parts = text.split(/(`[^`]+`)/g);

  return parts.map((part, index) => {
    // Inline code
    if (part.startsWith('`') && part.endsWith('`')) {
      return (
        <code key={index} className="bg-slate-100 px-1 py-0.5 rounded text-sm font-mono text-slate-700">
          {part.slice(1, -1)}
        </code>
      );
    }

    // Bold
    let result: React.ReactNode = part;
    if (part.includes('**')) {
      const boldParts = part.split(/(\*\*[^*]+\*\*)/g);
      result = boldParts.map((bp, i) => {
        if (bp.startsWith('**') && bp.endsWith('**')) {
          return <strong key={i}>{bp.slice(2, -2)}</strong>;
        }
        return bp;
      });
    }

    return <span key={index}>{result}</span>;
  });
}
