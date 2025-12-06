#!/usr/bin/env python3
"""
Test script to verify SPF/DKIM authentication integration
"""
import smtplib
import time
from email.mime.text import MIMEText
from email.mime.multipart import MIMEMultipart
import os
import glob

def send_test_email():
    """Send a test email to the mail server"""
    print("📧 Sending test email...")

    # Create a simple RFC 822 message
    message = """From: test@example.com
To: admin@delfour.co
Subject: SPF/DKIM Test Email
Date: Mon, 3 Dec 2025 12:00:00 +0000
Message-ID: <test-spf-dkim@example.com>

This is a test email to verify SPF/DKIM authentication.

The server should:
1. Validate SPF for the sender IP
2. Validate DKIM signature (if present)
3. Add Authentication-Results header
"""

    try:
        # Connect to SMTP server
        with smtplib.SMTP('localhost', 2525) as server:
            server.set_debuglevel(1)  # Enable debug output

            # Say EHLO
            server.ehlo('test-client.example.com')

            # Send MAIL FROM and RCPT TO
            server.mail('test@example.com')
            server.rcpt('admin@delfour.co')

            # Send message data
            server.data(message)

        print("✅ Email sent successfully!")
        return True

    except Exception as e:
        print(f"❌ Failed to send email: {e}")
        import traceback
        traceback.print_exc()
        return False

def check_authentication_header():
    """Check if the received email has Authentication-Results header"""
    print("\n🔍 Checking for Authentication-Results header...")

    # Wait a moment for email to be delivered
    time.sleep(1)

    # Find the latest email in admin mailbox
    maildir = "data/maildir/admin@delfour.co/new"
    # If not found, try with mail-rs/ prefix
    if not os.path.exists(maildir):
        maildir = "mail-rs/data/maildir/admin@delfour.co/new"

    if not os.path.exists(maildir):
        print(f"❌ Maildir not found: {maildir}")
        return False

    # Get the most recent email
    emails = glob.glob(f"{maildir}/*")
    if not emails:
        print("❌ No emails found in mailbox")
        return False

    latest_email = max(emails, key=os.path.getctime)
    print(f"📨 Reading email: {os.path.basename(latest_email)}")

    # Read email content
    with open(latest_email, 'r') as f:
        content = f.read()

    # Check for Authentication-Results header
    has_auth_header = 'Authentication-Results:' in content

    if has_auth_header:
        print("✅ Authentication-Results header found!")

        # Extract and display the header
        for line in content.split('\n'):
            if 'Authentication-Results:' in line or \
               (line.startswith(' ') and 'spf=' in line.lower()) or \
               (line.startswith(' ') and 'dkim=' in line.lower()):
                print(f"   {line.strip()}")

        # Check for SPF result
        has_spf = 'spf=' in content.lower()
        # Check for DKIM result
        has_dkim = 'dkim=' in content.lower()

        print(f"\n📊 Validation Results:")
        print(f"   SPF validated: {'✅' if has_spf else '❌'}")
        print(f"   DKIM validated: {'✅' if has_dkim else '❌'}")

        return True
    else:
        print("❌ Authentication-Results header NOT found")
        print("\n📄 Email headers (first 30 lines):")
        for i, line in enumerate(content.split('\n')[:30]):
            print(f"   {line}")
        return False

def main():
    print("=" * 60)
    print("SPF/DKIM Authentication Test")
    print("=" * 60)

    # Send test email
    if send_test_email():
        # Check if authentication header was added
        if check_authentication_header():
            print("\n" + "=" * 60)
            print("✅ SPF/DKIM INTEGRATION TEST PASSED!")
            print("=" * 60)
            return 0
        else:
            print("\n" + "=" * 60)
            print("❌ SPF/DKIM INTEGRATION TEST FAILED")
            print("   Authentication header not found in email")
            print("=" * 60)
            return 1
    else:
        print("\n" + "=" * 60)
        print("❌ SPF/DKIM INTEGRATION TEST FAILED")
        print("   Could not send test email")
        print("=" * 60)
        return 1

if __name__ == '__main__':
    exit(main())
