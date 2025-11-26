/**
 * Main App Component
 *
 * Simple hash-based routing for admin and chat interfaces
 */

import { useState, useEffect } from 'react';
import { Chat } from './components/Chat/Chat';
import { DashboardPage } from './pages/admin/DashboardPage';
import { UsersPage } from './pages/admin/UsersPage';

type Route = 'chat' | 'admin' | 'admin/users' | 'admin/settings';

function App() {
  const [currentRoute, setCurrentRoute] = useState<Route>('chat');

  useEffect(() => {
    // Parse initial hash
    const handleHashChange = () => {
      const hash = window.location.hash.slice(1); // Remove #

      if (hash.startsWith('/admin/users')) {
        setCurrentRoute('admin/users');
      } else if (hash.startsWith('/admin/settings')) {
        setCurrentRoute('admin/settings');
      } else if (hash.startsWith('/admin')) {
        setCurrentRoute('admin');
      } else {
        setCurrentRoute('chat');
      }
    };

    // Listen for hash changes
    window.addEventListener('hashchange', handleHashChange);
    handleHashChange(); // Initial load

    return () => {
      window.removeEventListener('hashchange', handleHashChange);
    };
  }, []);

  // Render based on route
  switch (currentRoute) {
    case 'admin':
      return <DashboardPage />;
    case 'admin/users':
      return <UsersPage />;
    case 'admin/settings':
      return (
        <div className="min-h-screen bg-gray-50 dark:bg-gray-900 flex items-center justify-center">
          <div className="text-center">
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white mb-4">
              Settings
            </h1>
            <p className="text-gray-600 dark:text-gray-400 mb-8">
              Settings page coming soon...
            </p>
            <a
              href="#/admin"
              className="inline-block bg-blue-600 hover:bg-blue-700 text-white font-medium px-4 py-2 rounded-lg transition-colors"
            >
              Back to Dashboard
            </a>
          </div>
        </div>
      );
    case 'chat':
    default:
      return (
        <div className="relative">
          <Chat />
          {/* Admin Link */}
          <a
            href="#/admin"
            className="fixed bottom-4 right-4 bg-gray-800 hover:bg-gray-700 text-white px-4 py-2 rounded-lg shadow-lg transition-colors text-sm font-medium"
            title="Admin Panel"
          >
            ⚙️ Admin
          </a>
        </div>
      );
  }
}

export default App;
