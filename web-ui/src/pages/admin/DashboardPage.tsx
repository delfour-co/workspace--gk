/**
 * DashboardPage - Admin Dashboard
 *
 * Overview of system statistics and health
 */

import { AdminLayout } from '../../components/admin/AdminLayout';
import { StatsCard } from '../../components/admin/StatsCard';
import { useStats } from '../../hooks/useStats';

export function DashboardPage() {
  const { stats, loading, error } = useStats();

  return (
    <AdminLayout currentPage="dashboard">
      <div className="space-y-6">
        {/* Header */}
        <div>
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
            Dashboard
          </h1>
          <p className="text-gray-600 dark:text-gray-400 mt-1">
            System overview and statistics
          </p>
        </div>

        {/* Error Alert */}
        {error && (
          <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-600 dark:text-red-400 px-4 py-3 rounded-lg">
            {error}
          </div>
        )}

        {/* Loading State */}
        {loading && (
          <div className="flex justify-center items-center p-8">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
          </div>
        )}

        {/* Statistics Cards */}
        {stats && !loading && (
          <>
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
              <StatsCard
                title="Total Users"
                value={stats.total_users}
                icon="👥"
                bgColor="bg-blue-500"
              />
              <StatsCard
                title="System Version"
                value={stats.version}
                icon="📦"
                bgColor="bg-green-500"
              />
              <StatsCard
                title="Status"
                value="Running"
                icon="✓"
                bgColor="bg-emerald-500"
              />
            </div>

            {/* Quick Actions */}
            <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
              <h2 className="text-xl font-bold text-gray-900 dark:text-white mb-4">
                Quick Actions
              </h2>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <a
                  href="#/admin/users"
                  className="flex items-center space-x-3 p-4 border border-gray-200 dark:border-gray-700 rounded-lg hover:border-blue-500 dark:hover:border-blue-500 transition-colors"
                >
                  <span className="text-2xl">👥</span>
                  <div>
                    <div className="font-medium text-gray-900 dark:text-white">
                      Manage Users
                    </div>
                    <div className="text-sm text-gray-600 dark:text-gray-400">
                      Create, edit, delete users
                    </div>
                  </div>
                </a>

                <a
                  href="#/"
                  className="flex items-center space-x-3 p-4 border border-gray-200 dark:border-gray-700 rounded-lg hover:border-blue-500 dark:hover:border-blue-500 transition-colors"
                >
                  <span className="text-2xl">💬</span>
                  <div>
                    <div className="font-medium text-gray-900 dark:text-white">
                      Email Chat
                    </div>
                    <div className="text-sm text-gray-600 dark:text-gray-400">
                      Go to chat interface
                    </div>
                  </div>
                </a>

                <div className="flex items-center space-x-3 p-4 border border-gray-200 dark:border-gray-700 rounded-lg opacity-50">
                  <span className="text-2xl">⚙️</span>
                  <div>
                    <div className="font-medium text-gray-900 dark:text-white">
                      Settings
                    </div>
                    <div className="text-sm text-gray-600 dark:text-gray-400">
                      Coming soon
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* System Info */}
            <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
              <h2 className="text-xl font-bold text-gray-900 dark:text-white mb-4">
                System Information
              </h2>
              <div className="space-y-3">
                <div className="flex justify-between py-2 border-b border-gray-200 dark:border-gray-700">
                  <span className="text-gray-600 dark:text-gray-400">Version</span>
                  <span className="font-medium text-gray-900 dark:text-white">{stats.version}</span>
                </div>
                <div className="flex justify-between py-2 border-b border-gray-200 dark:border-gray-700">
                  <span className="text-gray-600 dark:text-gray-400">Total Users</span>
                  <span className="font-medium text-gray-900 dark:text-white">{stats.total_users}</span>
                </div>
                <div className="flex justify-between py-2">
                  <span className="text-gray-600 dark:text-gray-400">Status</span>
                  <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400">
                    Operational
                  </span>
                </div>
              </div>
            </div>
          </>
        )}
      </div>
    </AdminLayout>
  );
}
