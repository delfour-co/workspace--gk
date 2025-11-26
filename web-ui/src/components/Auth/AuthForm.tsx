import { useState, type FormEvent } from 'react';

interface AuthFormProps {
  onAuthenticate: (email: string) => void;
  isConnected: boolean;
}

export function AuthForm({ onAuthenticate, isConnected }: AuthFormProps) {
  const [email, setEmail] = useState('');

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    if (email.trim() && email.includes('@')) {
      onAuthenticate(email.trim());
    }
  };

  return (
    <div className="flex items-center justify-center min-h-screen bg-white">
      <div className="w-full max-w-md p-8">
        <div className="text-center mb-10">
          <h1 className="text-3xl font-semibold text-slate-800 mb-2">
            GK Mail
          </h1>
          <p className="text-slate-600">
            Assistant IA pour vos emails
          </p>
        </div>

        {!isConnected ? (
          <div className="text-center py-12">
            <div className="inline-block animate-spin rounded-full h-10 w-10 border-3 border-slate-300 border-t-slate-800"></div>
            <p className="mt-4 text-slate-500">Connexion au serveur...</p>
          </div>
        ) : (
          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label htmlFor="email" className="block text-sm font-medium text-slate-700 mb-2">
                Adresse email
              </label>
              <input
                type="email"
                id="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="votre.email@exemple.com"
                className="w-full px-4 py-3 bg-white border border-slate-300 rounded-xl text-slate-800 placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-slate-400 focus:border-transparent transition"
                required
                autoFocus
              />
            </div>

            <button
              type="submit"
              disabled={!email.trim() || !email.includes('@')}
              className="w-full px-6 py-3 bg-slate-800 hover:bg-slate-700 disabled:bg-slate-300 disabled:cursor-not-allowed text-white font-medium rounded-xl transition"
            >
              Se connecter
            </button>

            <p className="text-xs text-slate-500 text-center mt-6">
              Entrez votre adresse email pour accéder à vos messages
            </p>
          </form>
        )}
      </div>
    </div>
  );
}
