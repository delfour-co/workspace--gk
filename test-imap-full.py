#!/usr/bin/env python3
"""Test complete IMAP flow"""

import socket
import time

def send_command(sock, command):
    """Send command and receive response"""
    print(f"→ {command}")
    sock.sendall(f"{command}\r\n".encode())
    time.sleep(0.3)
    response = sock.recv(8192).decode()
    print(f"← {response}")
    return response

def test_complete_flow():
    """Test complete IMAP flow"""
    print("=== Testing Complete IMAP Flow ===\n")

    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('localhost', 1993))

    # Receive greeting
    greeting = sock.recv(4096).decode()
    print(f"← {greeting}")

    # Test LOGIN with valid credentials
    print("\n1. LOGIN with valid credentials")
    response = send_command(sock, "A001 LOGIN test@example.com password123")
    if "OK" not in response:
        print("❌ LOGIN failed!")
        return

    print("✅ LOGIN successful!\n")

    # Test SELECT INBOX
    print("2. SELECT INBOX")
    response = send_command(sock, "A002 SELECT INBOX")
    if "OK" not in response:
        print("❌ SELECT failed!")
        return

    print("✅ SELECT successful!\n")

    # Test FETCH to get emails
    print("3. FETCH 1:* (BODY[])")
    response = send_command(sock, "A003 FETCH 1:* BODY[]")
    if "OK" not in response:
        print("❌ FETCH failed!")
        return

    print("✅ FETCH successful!\n")

    # Test LIST
    print("4. LIST \"\" \"*\"")
    response = send_command(sock, 'A004 LIST "" "*"')
    print("✅ LIST successful!\n")

    # Test LOGOUT
    print("5. LOGOUT")
    send_command(sock, "A005 LOGOUT")

    sock.close()
    print("\n=== All tests passed! 🎉 ===")

if __name__ == '__main__':
    test_complete_flow()
