#!/usr/bin/env python3
"""Test reading the E2E email via IMAP"""

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

def test_imap_read():
    """Test reading emails via IMAP"""
    print("=== Testing IMAP Email Reading ===\n")

    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('localhost', 1993))

    # Receive greeting
    greeting = sock.recv(4096).decode()
    print(f"← {greeting}")

    # LOGIN
    print("\n1. Logging in...")
    send_command(sock, "A001 LOGIN test@example.com password123")

    # SELECT INBOX
    print("\n2. Selecting INBOX...")
    response = send_command(sock, "A002 SELECT INBOX")

    # Extract message count
    import re
    match = re.search(r'\* (\d+) EXISTS', response)
    if match:
        count = int(match.group(1))
        print(f"\n✅ Found {count} emails in INBOX")

        # FETCH latest email
        print(f"\n3. Fetching email #{count} (latest)...")
        response = send_command(sock, f"A003 FETCH {count} BODY[]")

        # Check if E2E test email is present
        if "E2E Test Email" in response:
            print("\n🎉 SUCCESS! The E2E test email is readable via IMAP!")
            print("\nEmail content preview:")
            print("=" * 50)
            # Extract body
            if "full stack is working" in response:
                print("✅ E2E test message confirmed!")
        else:
            print("\n⚠️  Could not find E2E test email")

    # LOGOUT
    print("\n4. Logging out...")
    send_command(sock, "A004 LOGOUT")

    sock.close()
    print("\n=== IMAP Test Complete ===")

if __name__ == '__main__':
    test_imap_read()
