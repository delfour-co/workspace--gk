#!/usr/bin/env python3
"""Test IMAP server"""

import socket
import time

def send_command(sock, command):
    """Send command and receive response"""
    print(f"→ {command}")
    sock.sendall(f"{command}\r\n".encode())
    time.sleep(0.2)
    response = sock.recv(4096).decode()
    print(f"← {response}")
    return response

def test_imap():
    """Test IMAP server"""
    print("=== Testing IMAP Server ===\n")

    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('localhost', 1993))

    # Receive greeting
    greeting = sock.recv(4096).decode()
    print(f"← {greeting}")

    # Test CAPABILITY
    print("\n1. Testing CAPABILITY")
    send_command(sock, "A001 CAPABILITY")

    # Test NOOP
    print("\n2. Testing NOOP")
    send_command(sock, "A002 NOOP")

    # Test LOGIN (should fail - no user)
    print("\n3. Testing LOGIN (should fail)")
    send_command(sock, "A003 LOGIN test@example.com wrongpass")

    # Test LOGOUT
    print("\n4. Testing LOGOUT")
    send_command(sock, "A004 LOGOUT")

    sock.close()
    print("\n=== Tests completed ===")

if __name__ == '__main__':
    test_imap()
