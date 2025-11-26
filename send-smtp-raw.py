#!/usr/bin/env python3
"""Send email using raw SMTP protocol"""

import socket
import time
from datetime import datetime

def send_command(sock, command):
    """Send SMTP command and receive response"""
    print(f"→ {command}")
    sock.sendall(f"{command}\r\n".encode())
    time.sleep(0.2)
    response = sock.recv(4096).decode()
    print(f"← {response.strip()}")
    return response

def send_email():
    """Send test email via raw SMTP"""
    print("=== Sending Test Email via SMTP ===\n")

    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('localhost', 2525))

    # Receive greeting
    greeting = sock.recv(4096).decode()
    print(f"← {greeting.strip()}")

    # Send EHLO
    send_command(sock, "EHLO test-client")

    # Send MAIL FROM
    send_command(sock, "MAIL FROM:<sender@example.com>")

    # Send RCPT TO
    send_command(sock, "RCPT TO:<test@example.com>")

    # Send DATA
    send_command(sock, "DATA")

    # Send email content
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    email_content = f"""From: sender@example.com
To: test@example.com
Subject: E2E Test Email - {timestamp}
Date: {datetime.now().strftime('%a, %d %b %Y %H:%M:%S +0000')}

Hello!

This is an end-to-end test email sent via raw SMTP.

If you can read this in the web-ui via the AI assistant, it means:
✅ SMTP reception works
✅ Maildir storage works
✅ IMAP reading works
✅ MCP integration works
✅ AI runtime works
✅ Web-UI works

Congratulations! The full stack is working! 🎉

Best regards,
The GK Mail Test Suite
.
"""
    print("→ [Email content]")
    sock.sendall(email_content.encode())
    time.sleep(0.3)
    response = sock.recv(4096).decode()
    print(f"← {response.strip()}")

    # Send QUIT
    send_command(sock, "QUIT")

    sock.close()
    print("\n✅ Email sent successfully!")

if __name__ == '__main__':
    send_email()
