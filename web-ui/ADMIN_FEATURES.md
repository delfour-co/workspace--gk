# Admin UI Features

## Overview

Complete admin interface for managing the GK Mail system.

## Access

- **URL**: `http://localhost:5173/#/admin`
- **From Chat**: Click the "⚙️ Admin" button in the bottom-right corner

## Features Implemented

### 🏠 Dashboard (`/#/admin`)

**Statistics Cards**:
- Total Users
- System Version
- System Status

**Quick Actions**:
- Manage Users → Navigate to user management
- Email Chat → Return to chat interface
- Settings → Coming soon

**System Information**:
- Version display
- User count
- Operational status indicator

### 👥 User Management (`/#/admin/users`)

**User List**:
- Table with all users
- Columns: ID, Email, Created At
- Total user count display
- Responsive design

**Create User**:
- Modal form with validation
- Fields:
  - Email (validated format)
  - Password (min 8 characters)
  - Confirm Password
- Real-time error messages
- Success feedback

**Delete User**:
- Delete button for each user
- Confirmation dialog
- Immediate table update

### 🎨 Design

**Color Scheme**:
- Primary: Blue 600
- Success: Green 500
- Danger: Red 600
- Dark mode support throughout

**Layout**:
- Responsive sidebar navigation
- Clean card-based design
- Tailwind CSS styling
- Smooth transitions

## API Integration

**Endpoints Used**:
```
GET  /api/admin/users      - List all users
GET  /api/admin/users/:id  - Get user details
POST /api/admin/users      - Create user
DELETE /api/admin/users/:id - Delete user
GET  /api/admin/stats      - System statistics
```

**Authentication**:
- JWT token stored in localStorage
- Auto-includes Bearer token in requests
- Logout clears token

## Usage

### Development
```bash
cd web-ui
npm run dev
```

Access at: `http://localhost:5173`

### Production Build
```bash
npm run build
```

Output in `web-ui/dist/`

### Environment Variables

Create `.env` file:
```env
VITE_API_URL=http://localhost:8080
```

Default: `http://localhost:8080`

## Navigation

**Routes** (hash-based):
- `/#/` - Chat interface
- `/#/admin` - Admin dashboard
- `/#/admin/users` - User management
- `/#/admin/settings` - Settings (placeholder)

**Sidebar Navigation**:
- 📊 Dashboard
- 👥 Users
- ⚙️ Settings

## Components

### Admin Components

1. **AdminLayout** (`components/admin/AdminLayout.tsx`)
   - Main layout with sidebar
   - Header with logout
   - Navigation items

2. **UserTable** (`components/admin/UserTable.tsx`)
   - Displays users in table
   - Loading state
   - Empty state
   - Delete action

3. **CreateUserForm** (`components/admin/CreateUserForm.tsx`)
   - Modal dialog
   - Form validation
   - Error handling
   - Cancel/Submit actions

4. **StatsCard** (`components/admin/StatsCard.tsx`)
   - Statistic display card
   - Icon, title, value
   - Customizable background

### Pages

1. **DashboardPage** (`pages/admin/DashboardPage.tsx`)
   - Stats overview
   - Quick actions
   - System info

2. **UsersPage** (`pages/admin/UsersPage.tsx`)
   - User management
   - Create user button
   - User table
   - Create form modal

### Hooks

1. **useUsers** (`hooks/useUsers.ts`)
   - Fetch users
   - Create user
   - Delete user
   - Error handling

2. **useStats** (`hooks/useStats.ts`)
   - Fetch statistics
   - Loading state
   - Refetch capability

### API Client

**File**: `lib/api.ts`

**Functions**:
- `authApi.login()` - Authenticate
- `usersApi.list()` - Get all users
- `usersApi.create()` - Create user
- `usersApi.delete()` - Delete user
- `statsApi.get()` - Get statistics

**Token Management**:
- `setAuthToken(token)` - Store JWT
- `getAuthToken()` - Retrieve JWT
- `clearAuthToken()` - Remove JWT

## Security

- JWT authentication required
- Token auto-included in headers
- Logout clears all auth data
- API error handling

## Future Enhancements

### Planned Features
- [ ] User edit functionality
- [ ] User search and filtering
- [ ] Pagination for large user lists
- [ ] User roles (admin, user, read-only)
- [ ] Settings page
- [ ] System logs viewer
- [ ] Email queue management
- [ ] Real-time metrics charts
- [ ] Bulk user operations
- [ ] User export/import

### Technical Improvements
- [ ] React Router for better routing
- [ ] State management (Zustand/Context)
- [ ] Real-time updates (WebSocket)
- [ ] Toast notifications library
- [ ] Form validation library
- [ ] Loading skeletons
- [ ] Table sorting and filtering
- [ ] Infinite scroll for users

## Screenshots

### Dashboard
- Statistics cards with icons
- Quick action links
- System information panel

### User Management
- Sortable user table
- Create user button (top-right)
- Delete confirmation
- Modal form for user creation

## Testing

Run development server and test:

1. **Dashboard**:
   - Navigate to `/#/admin`
   - Check stats display
   - Click quick actions

2. **User Management**:
   - Navigate to `/#/admin/users`
   - Click "Create User"
   - Fill form and submit
   - Verify user appears in table
   - Test delete functionality

3. **Navigation**:
   - Test sidebar links
   - Test hash routing
   - Test logout

## Troubleshooting

**Issue**: "Failed to fetch users"
- Check API is running on port 8080
- Verify JWT token is set
- Check network tab for errors

**Issue**: "Type errors during build"
- Ensure all type imports use `import type`
- Check tsconfig.json settings

**Issue**: "Routing not working"
- Use hash URLs: `/#/admin`
- Check browser console for errors
- Verify hash change listener

## Development Notes

- Uses hash-based routing (no React Router dependency)
- TypeScript strict mode enabled
- Tailwind CSS for styling
- No external UI library (pure React + Tailwind)
- Responsive design mobile-first

## File Structure

```
web-ui/src/
├── components/
│   └── admin/
│       ├── AdminLayout.tsx      # Main layout
│       ├── UserTable.tsx        # User list
│       ├── CreateUserForm.tsx   # Create form
│       └── StatsCard.tsx        # Stat display
├── pages/
│   └── admin/
│       ├── DashboardPage.tsx    # Dashboard
│       └── UsersPage.tsx        # Users page
├── hooks/
│   ├── useUsers.ts              # User CRUD
│   └── useStats.ts              # Statistics
├── lib/
│   └── api.ts                   # API client
└── App.tsx                      # Router
```

## Performance

- Build size: ~228 KB (gzipped: 69 KB)
- Initial load: Fast
- Route changes: Instant (hash-based)
- API calls: Optimized with proper loading states

## Browser Support

- Chrome/Edge: ✅
- Firefox: ✅
- Safari: ✅
- Mobile browsers: ✅

## License

Part of GK Mail project
