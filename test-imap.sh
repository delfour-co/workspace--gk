#!/bin/bash
# Test IMAP server

echo "=== Test IMAP Server ==="
echo

echo "1. Testing CAPABILITY"
(
  sleep 0.5
  echo "A001 CAPABILITY"
  sleep 0.5
) | nc localhost 1993
echo

echo "2. Testing LOGIN (should fail - no user)"
(
  sleep 0.5
  echo "A002 LOGIN test@example.com wrongpassword"
  sleep 0.5
) | nc localhost 1993
echo

echo "3. Testing LOGOUT"
(
  sleep 0.5
  echo "A003 LOGOUT"
  sleep 0.5
) | nc localhost 1993
echo

echo "=== Tests completed ==="
