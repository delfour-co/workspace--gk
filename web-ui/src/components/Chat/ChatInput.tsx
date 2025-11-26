import { useState, type KeyboardEvent } from 'react';

interface ChatInputProps {
  onSend: (message: string) => void;
  disabled?: boolean;
}

export function ChatInput({ onSend, disabled = false }: ChatInputProps) {
  const [input, setInput] = useState('');

  const handleSend = () => {
    const message = input.trim();
    if (!message || disabled) return;

    onSend(message);
    setInput('');
  };

  const handleKeyPress = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="flex gap-2 items-end">
      <input
        type="text"
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyPress={handleKeyPress}
        placeholder="Posez une question sur vos emails..."
        disabled={disabled}
        className="flex-1 px-4 py-3 rounded-xl border border-slate-300
                   bg-white text-slate-800
                   focus:outline-none focus:ring-2 focus:ring-slate-400 focus:border-transparent
                   disabled:opacity-50 disabled:cursor-not-allowed
                   placeholder:text-slate-400"
      />
      <button
        onClick={handleSend}
        disabled={disabled || !input.trim()}
        className="px-5 py-3 bg-slate-800 hover:bg-slate-700 text-white rounded-xl
                   font-medium transition-colors disabled:opacity-30
                   disabled:cursor-not-allowed disabled:hover:bg-slate-800"
      >
        →
      </button>
    </div>
  );
}
