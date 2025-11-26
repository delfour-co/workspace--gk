/**
 * Admin Layout Component
 *
 * Provides the main layout for admin pages with navigation
 */

import type { ReactNode } from 'react';

interface AdminLayoutProps {
  children: ReactNode;
  currentPage?: 'dashboard' | 'users' | 'settings';
}

export function AdminLayout({ children, currentPage = 'dashboard' }: AdminLayoutProps) {
  const navItems = [
    { id: 'dashboard', label: 'Dashboard', icon: '📊', href: '#/admin' },
    { id: 'users', label: 'Users', icon: '👥', href: '#/admin/users' },
    { id: 'settings', label: 'Settings', icon: '⚙️', href: '#/admin/settings' },
  ];

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900">
      {/* Header */}
      <header className="bg-white dark:bg-gray-800 shadow-sm border-b border-gray-200 dark:border-gray-700">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex justify-between items-center h-16">
            <div className="flex items-center space-x-3">
              <h1 className="text-xl font-bold text-gray-900 dark:text-white">
                GK Mail Admin
              </h1>
            </div>
            <div className="flex items-center space-x-4">
              <button
                onClick={() => {
                  localStorage.removeItem('auth_token');
                  window.location.hash = '#/';
                }}
                className="text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white"
              >
                Logout
              </button>
            </div>
          </div>
        </div>
      </header>

      <div className="flex">
        {/* Sidebar */}
        <aside className="w-64 bg-white dark:bg-gray-800 min-h-[calc(100vh-4rem)] border-r border-gray-200 dark:border-gray-700">
          <nav className="p-4 space-y-2">
            {navItems.map((item) => (
              <a
                key={item.id}
                href={item.href}
                className={`flex items-center space-x-3 px-4 py-3 rounded-lg transition-colors ${
                  currentPage === item.id
                    ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400'
                    : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'
                }`}
              >
                <span className="text-xl">{item.icon}</span>
                <span className="font-medium">{item.label}</span>
              </a>
            ))}
          </nav>
        </aside>

        {/* Main Content */}
        <main className="flex-1 p-8">
          <div className="max-w-7xl mx-auto">
            {children}
          </div>
        </main>
      </div>
    </div>
  );
}
