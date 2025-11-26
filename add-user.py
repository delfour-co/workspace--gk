#!/usr/bin/env python3
"""Add a test user to the database"""

import sqlite3
from argon2 import PasswordHasher
from datetime import datetime

# Connect to database
conn = sqlite3.connect('/home/kdelfour/Workspace/Personnel/Prototype/gk/mail.db')
cursor = conn.cursor()

# Create table if not exists (should already exist)
cursor.execute('''
    CREATE TABLE IF NOT EXISTS smtp_users (
        email TEXT PRIMARY KEY,
        password_hash TEXT NOT NULL,
        created_at TEXT NOT NULL,
        last_login TEXT
    )
''')

# Hash password
ph = PasswordHasher()
email = "test@example.com"
password = "password123"
password_hash = ph.hash(password)

# Insert user
try:
    cursor.execute('''
        INSERT INTO smtp_users (email, password_hash, created_at)
        VALUES (?, ?, ?)
    ''', (email, password_hash, datetime.now().isoformat()))
    conn.commit()
    print(f"✅ User added: {email}")
    print(f"   Password: {password}")
except sqlite3.IntegrityError:
    print(f"⚠️  User {email} already exists")

# List all users
cursor.execute('SELECT email, created_at FROM smtp_users')
users = cursor.fetchall()
print(f"\nAll users in database:")
for user in users:
    print(f"  - {user[0]} (created: {user[1]})")

conn.close()
