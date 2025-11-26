import { useState } from 'react';
import type { Email } from '../../types';

interface EmailCardProps {
  email: Email;
  expanded?: boolean;
}

export function EmailCard({ email, expanded: initialExpanded = false }: EmailCardProps) {
  const [expanded, setExpanded] = useState(initialExpanded);

  // Format date to readable string
  const formatDate = (dateStr: string) => {
    if (!dateStr) return '';
    try {
      const date = new Date(dateStr);
      const now = new Date();
      const isToday = date.toDateString() === now.toDateString();

      if (isToday) {
        return date.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' });
      }
      return date.toLocaleDateString('fr-FR', { day: 'numeric', month: 'short' });
    } catch {
      return dateStr;
    }
  };

  // Extract sender name from email address
  const getSenderName = (from: string) => {
    // Handle "Name <email@domain.com>" format
    const match = from.match(/^([^<]+)</);
    if (match) {
      return match[1].trim().replace(/"/g, '');
    }
    // Handle plain email
    return from.split('@')[0];
  };

  // Check if email has unread flag
  const isUnread = !email.flags?.includes('\\Seen');

  return (
    <div
      className={`border rounded-lg overflow-hidden transition-all ${
        expanded ? 'bg-white shadow-md' : 'bg-slate-50 hover:bg-white hover:shadow-sm'
      }`}
    >
      {/* Header - always visible */}
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full text-left p-3 flex items-start gap-3"
      >
        {/* Unread indicator */}
        <div className="flex-shrink-0 mt-1">
          {isUnread ? (
            <div className="w-2 h-2 rounded-full bg-blue-500" />
          ) : (
            <div className="w-2 h-2" />
          )}
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-baseline justify-between gap-2">
            <span className={`truncate ${isUnread ? 'font-semibold text-slate-900' : 'text-slate-700'}`}>
              {getSenderName(email.from)}
            </span>
            <span className="flex-shrink-0 text-xs text-slate-400">
              {formatDate(email.date)}
            </span>
          </div>
          <div className={`truncate text-sm ${isUnread ? 'font-medium text-slate-800' : 'text-slate-600'}`}>
            {email.subject}
          </div>
          {!expanded && email.preview && (
            <div className="truncate text-xs text-slate-400 mt-0.5">
              {email.preview}
            </div>
          )}
        </div>

        {/* Expand icon */}
        <div className="flex-shrink-0 text-slate-400">
          <svg
            className={`w-4 h-4 transition-transform ${expanded ? 'rotate-180' : ''}`}
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
          </svg>
        </div>
      </button>

      {/* Expanded content */}
      {expanded && (
        <div className="px-3 pb-3 border-t border-slate-100">
          {/* Full email details */}
          <div className="mt-3 space-y-2 text-sm">
            <div className="flex gap-2">
              <span className="text-slate-400 w-12">De:</span>
              <span className="text-slate-700">{email.from}</span>
            </div>
            <div className="flex gap-2">
              <span className="text-slate-400 w-12">À:</span>
              <span className="text-slate-700">{email.to}</span>
            </div>
            <div className="flex gap-2">
              <span className="text-slate-400 w-12">Date:</span>
              <span className="text-slate-700">{email.date}</span>
            </div>
          </div>

          {/* Email body */}
          {email.body && (
            <div className="mt-4 p-3 bg-slate-50 rounded text-sm text-slate-700 whitespace-pre-wrap">
              {email.body}
            </div>
          )}

          {/* Flags */}
          {email.flags && email.flags.length > 0 && (
            <div className="mt-3 flex gap-1 flex-wrap">
              {email.flags.map((flag, i) => (
                <span key={i} className="px-2 py-0.5 bg-slate-100 text-slate-500 text-xs rounded">
                  {flag.replace('\\', '')}
                </span>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

interface EmailListProps {
  emails: Email[];
  title?: string;
}

export function EmailList({ emails, title }: EmailListProps) {
  if (emails.length === 0) {
    return (
      <div className="text-center py-4 text-slate-400 text-sm">
        Aucun email trouvé
      </div>
    );
  }

  return (
    <div className="space-y-2">
      {title && (
        <div className="text-xs text-slate-500 font-medium mb-2">
          {title} ({emails.length})
        </div>
      )}
      {emails.map((email, index) => (
        <EmailCard key={email.id || index} email={email} />
      ))}
    </div>
  );
}
