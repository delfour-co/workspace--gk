#!/usr/bin/env python3
"""Fetch all emails via IMAP to find E2E test"""

import socket
import time

def send_command(sock, command):
    """Send command and receive response"""
    print(f"→ {command}")
    sock.sendall(f"{command}\r\n".encode())
    time.sleep(0.3)
    response = sock.recv(16384).decode()
    return response

def test_imap_all():
    """Test reading all emails via IMAP"""
    print("=== Fetching All Emails via IMAP ===\n")

    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('localhost', 1993))

    # Receive greeting
    greeting = sock.recv(4096).decode()
    print(f"← {greeting}")

    # LOGIN
    print("1. Logging in...")
    send_command(sock, "A001 LOGIN test@example.com password123")

    # SELECT INBOX
    print("2. Selecting INBOX...")
    response = send_command(sock, "A002 SELECT INBOX")
    print(f"← {response}")

    # FETCH all emails
    print("\n3. Fetching ALL emails (1:*)...")
    response = send_command(sock, "A003 FETCH 1:* BODY[]")

    # Check for E2E test email
    if "E2E Test Email" in response and "full stack is working" in response:
        print("\n🎉 SUCCESS! The E2E test email is readable via IMAP!")
        print("\n✅ SMTP → Maildir → IMAP chain works perfectly!")

        # Count emails
        email_count = response.count("* ") - 1  # Minus the OK response
        print(f"\n📬 Total emails fetched: {email_count}")
    else:
        print("\n⚠️  E2E test email not found in response")
        print("\nResponse preview:")
        print(response[:500])

    # LOGOUT
    print("\n4. Logging out...")
    send_command(sock, "A004 LOGOUT")

    sock.close()
    print("\n=== Test Complete ===")

if __name__ == '__main__':
    test_imap_all()
