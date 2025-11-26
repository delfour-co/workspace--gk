#!/usr/bin/env python3
"""Send a test email via SMTP"""

import smtplib
from email.mime.text import MIMEText
from email.mime.multipart import MIMEMultipart
from datetime import datetime

def send_email():
    # Email configuration
    smtp_host = 'localhost'
    smtp_port = 2525
    from_addr = 'sender@example.com'
    to_addr = 'test@example.com'

    # Create message
    msg = MIMEMultipart()
    msg['From'] = from_addr
    msg['To'] = to_addr
    msg['Subject'] = f'E2E Test Email - {datetime.now().strftime("%Y-%m-%d %H:%M:%S")}'
    msg['Date'] = datetime.now().strftime('%a, %d %b %Y %H:%M:%S +0000')

    body = """Hello!

This is an end-to-end test email sent via SMTP.

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
"""

    msg.attach(MIMEText(body, 'plain'))

    # Send email
    try:
        print(f"Connecting to SMTP server at {smtp_host}:{smtp_port}...")
        server = smtplib.SMTP(smtp_host, smtp_port, timeout=10)

        print("Sending EHLO...")
        server.ehlo()

        print(f"Sending email from {from_addr} to {to_addr}...")
        server.sendmail(from_addr, [to_addr], msg.as_string())

        print("✅ Email sent successfully!")
        print(f"   From: {from_addr}")
        print(f"   To: {to_addr}")
        print(f"   Subject: {msg['Subject']}")

        server.quit()
        return True

    except Exception as e:
        print(f"❌ Failed to send email: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == '__main__':
    send_email()
