#!/usr/bin/env python3
"""
Generate synthetic test emails with fake addresses for testing.
Uses real mail images from results/ but generates fictional recipient addresses.
"""

import base64
import email
import json
import os
import sys
from datetime import datetime
from email.mime.image import MIMEImage
from email.mime.multipart import MIMEMultipart
from email.mime.text import MIMEText
from pathlib import Path

def create_email_with_images(to_addr, subject, images):
    """
    Create a MIME email with attached images.

    Args:
        to_addr: Recipient address dict with name, street, city, state, zip
        subject: Email subject
        images: List of (filepath, content_id) tuples

    Returns:
        Email as string
    """
    msg = MIMEMultipart('related')
    msg['From'] = 'USPS Informed Delivery <USPSInformedDelivery@usps.gov>'
    msg['To'] = f"{to_addr['name']} <test@example.com>"
    msg['Subject'] = subject
    msg['Date'] = datetime.now().strftime('%a, %d %b %Y %H:%M:%S +0000')
    msg['Message-ID'] = f'<{datetime.now().timestamp()}.synthetic@email.informeddelivery.usps.gov>'

    # Add text body
    body = MIMEText('Your Daily Digest for mail is ready to view', 'plain')
    msg.attach(body)

    # Add images
    for filepath, content_id in images:
        with open(filepath, 'rb') as f:
            img_data = f.read()
        img = MIMEImage(img_data, name=Path(filepath).name)
        img.add_header('Content-ID', f'<{content_id}>')
        img.add_header('Content-Disposition', 'inline', filename=Path(filepath).name)
        msg.attach(img)

    return msg.as_string()

def main():
    # Fake addresses for testing
    test_cases = [
        {
            'name': 'mailer_and_content',
            'to_addr': {
                'name': 'John Smith',
                'street': '123 Main Street',
                'city': 'Springfield',
                'state': 'IL',
                'zip': '62701'
            },
            'subject': 'Your Daily Digest for Mon, 6/2 is ready to view',
            'images': [
                ('results/ff6c29d555edc72a/3d441051a60faabb/mailer.jpg', 'mailer-001'),
                ('results/ff6c29d555edc72a/3d441051a60faabb/content.jpg', 'ra_0_001'),
            ]
        },
        {
            'name': 'mailer_only',
            'to_addr': {
                'name': 'Jane Doe',
                'street': '456 Oak Avenue',
                'city': 'Portland',
                'state': 'OR',
                'zip': '97201'
            },
            'subject': 'Your Daily Digest for Tue, 6/3 is ready to view',
            'images': [
                ('results/997c85aabce87875/00c4709de774d8ea/mailer.jpg', 'mailer-002'),
            ]
        },
        {
            'name': 'no_images',
            'to_addr': {
                'name': 'Bob Johnson',
                'street': '789 Elm Drive',
                'city': 'Austin',
                'state': 'TX',
                'zip': '78701'
            },
            'subject': 'Your Expected Delivery for Wed, 6/4',
            'images': []
        },
    ]

    emails_dir = Path('../emails')
    emails_dir.mkdir(exist_ok=True)

    for case in test_cases:
        print(f"Generating {case['name']}.eml...")
        email_content = create_email_with_images(
            case['to_addr'],
            case['subject'],
            case['images']
        )

        output_path = emails_dir / f"{case['name']}.eml"
        with open(output_path, 'w') as f:
            f.write(email_content)

        print(f"  -> {output_path}")

    print(f"\nGenerated {len(test_cases)} test emails in {emails_dir}")

if __name__ == '__main__':
    main()
