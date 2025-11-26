/**
 * UsersPage - User Management Page
 *
 * Main page for managing users
 */

import { useState } from 'react';
import { AdminLayout } from '../../components/admin/AdminLayout';
import { UserTable } from '../../components/admin/UserTable';
import { CreateUserForm } from '../../components/admin/CreateUserForm';
import { useUsers } from '../../hooks/useUsers';

export function UsersPage() {
  const { users, loading, error, createUser, deleteUser } = useUsers();
  const [showCreateForm, setShowCreateForm] = useState(false);

  return (
    <AdminLayout currentPage="users">
      <div className="space-y-6">
        {/* Header */}
        <div className="flex justify-between items-center">
          <div>
            <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
              Users
            </h1>
            <p className="text-gray-600 dark:text-gray-400 mt-1">
              Manage user accounts
            </p>
          </div>
          <button
            onClick={() => setShowCreateForm(true)}
            className="bg-blue-600 hover:bg-blue-700 text-white font-medium px-4 py-2 rounded-lg transition-colors flex items-center space-x-2"
          >
            <span>+</span>
            <span>Create User</span>
          </button>
        </div>

        {/* Error Alert */}
        {error && (
          <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-600 dark:text-red-400 px-4 py-3 rounded-lg">
            {error}
          </div>
        )}

        {/* User Count */}
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-4">
          <div className="text-sm text-gray-600 dark:text-gray-400">
            Total Users: <span className="font-bold text-gray-900 dark:text-white">{users.length}</span>
          </div>
        </div>

        {/* User Table */}
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
          <UserTable
            users={users}
            onDelete={deleteUser}
            loading={loading}
          />
        </div>
      </div>

      {/* Create User Modal */}
      {showCreateForm && (
        <CreateUserForm
          onSubmit={createUser}
          onCancel={() => setShowCreateForm(false)}
        />
      )}
    </AdminLayout>
  );
}
