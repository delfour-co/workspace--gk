# 🛠️ Admin UI Implementation Plan

## Overview

Admin interface for managing the GK Mail system - users, configuration, and monitoring.

## Architecture

```
web-ui/
├── src/
│   ├── pages/
│   │   ├── Admin/
│   │   │   ├── Dashboard.tsx      # System overview
│   │   │   ├── Users.tsx          # User management
│   │   │   ├── UserCreate.tsx     # Create user form
│   │   │   ├── UserEdit.tsx       # Edit user form
│   │   │   └── Settings.tsx       # System configuration
│   │   └── Chat.tsx               # Existing chat interface
│   ├── components/
│   │   ├── admin/
│   │   │   ├── UserTable.tsx      # User list table
│   │   │   ├── StatsCard.tsx      # Statistics card
│   │   │   ├── SystemStatus.tsx   # System health status
│   │   │   └── AdminLayout.tsx    # Admin page layout
│   │   └── chat/                  # Existing chat components
│   ├── hooks/
│   │   ├── useAuth.ts             # Authentication hook
│   │   ├── useUsers.ts            # User management hook
│   │   └── useStats.ts            # Statistics hook
│   └── lib/
│       ├── api.ts                 # API client
│       └── types.ts               # TypeScript types
```

## API Endpoints (Backend)

### User Management
- `GET /api/admin/users` - List all users
- `GET /api/admin/users/:id` - Get user details
- `POST /api/admin/users` - Create user
- `PATCH /api/admin/users/:id` - Update user
- `DELETE /api/admin/users/:id` - Delete user

### System
- `GET /api/admin/stats` - Get system statistics
- `GET /api/admin/config` - Get configuration
- `GET /api/health` - Health check

## Pages

### 1. Dashboard (`/admin`)
**Components**:
- System statistics cards (total users, active users, etc.)
- Health status indicators
- Recent activity log
- Quick actions

**Stats Cards**:
```tsx
<StatsCard
  title="Total Users"
  value={stats.totalUsers}
  icon={<Users />}
  trend={+5}
/>
```

### 2. User Management (`/admin/users`)
**Features**:
- Searchable user table
- Pagination (20 per page)
- Sort by date, email
- Create/Edit/Delete actions
- Quick filters (active/inactive)

**Table Columns**:
- ID
- Email
- Created At
- Last Login (if available)
- Actions (Edit, Delete)

### 3. User Create/Edit (`/admin/users/new`, `/admin/users/:id/edit`)
**Form Fields**:
- Email (required, validated)
- Password (required for create, optional for edit)
- Full Name (optional)
- Active Status (checkbox)

**Validation**:
- Email format
- Password strength (min 8 chars)
- Duplicate email check

### 4. System Settings (`/admin/settings`)
**Sections**:
- Server Configuration (read-only for now)
- Version Information
- Security Settings

## Components

### AdminLayout
```tsx
<AdminLayout>
  <Sidebar>
    <NavLink to="/admin">Dashboard</NavLink>
    <NavLink to="/admin/users">Users</NavLink>
    <NavLink to="/admin/settings">Settings</NavLink>
  </Sidebar>
  <MainContent>{children}</MainContent>
</AdminLayout>
```

### UserTable
```tsx
<UserTable
  users={users}
  onEdit={(user) => navigate(`/admin/users/${user.id}/edit`)}
  onDelete={(user) => confirmDelete(user)}
  loading={loading}
/>
```

### StatsCard
```tsx
<StatsCard
  title="System Statistic"
  value={123}
  icon={<Icon />}
  change="+5%"
  changeType="positive"
/>
```

## Routing

```tsx
<Routes>
  {/* Public */}
  <Route path="/" element={<Chat />} />
  <Route path="/login" element={<Login />} />

  {/* Admin (protected) */}
  <Route path="/admin" element={<AdminLayout />}>
    <Route index element={<Dashboard />} />
    <Route path="users" element={<Users />} />
    <Route path="users/new" element={<UserCreate />} />
    <Route path="users/:id/edit" element={<UserEdit />} />
    <Route path="settings" element={<Settings />} />
  </Route>
</Routes>
```

## State Management

### API Client (`lib/api.ts`)
```typescript
export const api = {
  users: {
    list: () => axios.get('/api/admin/users'),
    get: (id) => axios.get(`/api/admin/users/${id}`),
    create: (data) => axios.post('/api/admin/users', data),
    update: (id, data) => axios.patch(`/api/admin/users/${id}`, data),
    delete: (id) => axios.delete(`/api/admin/users/${id}`),
  },
  stats: {
    get: () => axios.get('/api/admin/stats'),
  },
};
```

### Custom Hooks

**useUsers**:
```typescript
const useUsers = () => {
  const [users, setUsers] = useState([]);
  const [loading, setLoading] = useState(false);

  const fetchUsers = async () => {
    setLoading(true);
    const { data } = await api.users.list();
    setUsers(data);
    setLoading(false);
  };

  return { users, loading, fetchUsers, ... };
};
```

**useStats**:
```typescript
const useStats = () => {
  const [stats, setStats] = useState(null);

  const fetchStats = async () => {
    const { data } = await api.stats.get();
    setStats(data);
  };

  return { stats, fetchStats };
};
```

## Styling

**Tailwind CSS classes**:
- Admin layout: `bg-gray-50 dark:bg-gray-900`
- Sidebar: `w-64 bg-white dark:bg-gray-800`
- Cards: `bg-white dark:bg-gray-800 rounded-lg shadow`
- Tables: `divide-y divide-gray-200 dark:divide-gray-700`
- Buttons: `bg-blue-600 hover:bg-blue-700 text-white`

**Color Palette**:
- Primary: Blue (`blue-600`)
- Success: Green (`green-600`)
- Danger: Red (`red-600`)
- Warning: Yellow (`yellow-500`)
- Gray scale: `gray-50` to `gray-900`

## Security

### Authentication
- JWT token in localStorage/sessionStorage
- Axios interceptor for Authorization header
- Redirect to login if 401
- Admin role check (future)

### Authorization
- Admin-only routes (protected)
- User permission checks
- CSRF protection
- Rate limiting

## Implementation Steps

### Phase 1: Basic Structure (2 hours)
1. Create admin page components
2. Add routing for admin pages
3. Create AdminLayout component
4. Setup API client

### Phase 2: User Management (3 hours)
1. Implement UserTable component
2. Create user list page with search/pagination
3. Build user create form
4. Build user edit form
5. Add delete confirmation dialog

### Phase 3: Dashboard (2 hours)
1. Create StatsCard component
2. Fetch and display statistics
3. Add system health indicators
4. Add quick action buttons

### Phase 4: Polish (1 hour)
1. Add loading states
2. Error handling and toasts
3. Form validation
4. Responsive design
5. Dark mode support

## Testing

- Test user CRUD operations
- Test form validation
- Test error handling
- Test pagination
- Test search functionality
- Test responsive layout

## Future Enhancements

- **Logs Viewer**: Real-time log streaming
- **Metrics Dashboard**: Grafana-style charts
- **Email Queue**: View pending/failed emails
- **Config Editor**: Edit TOML config via UI
- **User Roles**: Admin, User, Read-only
- **Audit Log**: Track all admin actions
- **Backup/Restore**: Database operations
- **Bulk Operations**: Batch user import/export

## API Response Examples

### List Users
```json
[
  {
    "id": 1,
    "email": "admin@example.com",
    "created_at": "2025-11-26T10:00:00Z"
  }
]
```

### System Stats
```json
{
  "total_users": 42,
  "version": "0.1.0"
}
```

### Create User Request
```json
{
  "email": "newuser@example.com",
  "password": "securepassword123"
}
```

## Notes

- Admin interface accessible at `/admin`
- Requires authentication (JWT token)
- All admin endpoints under `/api/admin/*`
- Use existing auth system from web-ui
- Maintain consistent design with chat interface
- Mobile-responsive by default
