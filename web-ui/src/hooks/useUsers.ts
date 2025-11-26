/**
 * useUsers Hook
 *
 * Manages user data and CRUD operations
 */

import { useState, useEffect } from 'react';
import { usersApi } from '../lib/api';
import type { User, CreateUserRequest } from '../lib/api';

export function useUsers() {
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchUsers = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await usersApi.list();
      setUsers(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch users');
    } finally {
      setLoading(false);
    }
  };

  const createUser = async (data: CreateUserRequest): Promise<boolean> => {
    setLoading(true);
    setError(null);
    try {
      const newUser = await usersApi.create(data);
      setUsers([newUser, ...users]);
      return true;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create user');
      return false;
    } finally {
      setLoading(false);
    }
  };

  const deleteUser = async (id: number): Promise<boolean> => {
    if (!confirm('Are you sure you want to delete this user?')) {
      return false;
    }

    setLoading(true);
    setError(null);
    try {
      await usersApi.delete(id);
      setUsers(users.filter((u) => u.id !== id));
      return true;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete user');
      return false;
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchUsers();
  }, []);

  return {
    users,
    loading,
    error,
    fetchUsers,
    createUser,
    deleteUser,
  };
}
