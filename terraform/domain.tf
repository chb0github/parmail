data "aws_route53_zone" "parent" {
  name = var.parent_domain
}

resource "aws_route53_zone" "parmail" {
  name = "parmail.${var.parent_domain}"
}

resource "aws_route53_record" "subdomain_ns" {
  zone_id = data.aws_route53_zone.parent.zone_id
  name    = "parmail.${var.parent_domain}"
  type    = "NS"
  ttl     = 300
  records = aws_route53_zone.parmail.name_servers
}

resource "aws_route53_record" "mx" {
  zone_id = aws_route53_zone.parmail.zone_id
  name    = "parmail.${var.parent_domain}"
  type    = "MX"
  ttl     = 300
  records = ["10 inbound-smtp.${var.aws_region}.amazonaws.com"]
}

resource "aws_ses_domain_identity" "parmail" {
  domain = "parmail.${var.parent_domain}"
}

resource "aws_route53_record" "ses_verification" {
  zone_id = aws_route53_zone.parmail.zone_id
  name    = "_amazonses.parmail.${var.parent_domain}"
  type    = "TXT"
  ttl     = 300
  records = [aws_ses_domain_identity.parmail.verification_token]
}

resource "aws_ses_domain_identity_verification" "parmail" {
  domain     = aws_ses_domain_identity.parmail.id
  depends_on = [aws_route53_record.ses_verification]
}

# --- DKIM for sending ---

resource "aws_ses_domain_dkim" "parmail" {
  domain = aws_ses_domain_identity.parmail.domain
}

resource "aws_route53_record" "ses_dkim" {
  count   = 3
  zone_id = aws_route53_zone.parmail.zone_id
  name    = "${aws_ses_domain_dkim.parmail.dkim_tokens[count.index]}._domainkey.parmail.${var.parent_domain}"
  type    = "CNAME"
  ttl     = 300
  records = ["${aws_ses_domain_dkim.parmail.dkim_tokens[count.index]}.dkim.amazonses.com"]
}

# --- SPF record for sending ---

resource "aws_route53_record" "spf" {
  zone_id = aws_route53_zone.parmail.zone_id
  name    = "parmail.${var.parent_domain}"
  type    = "TXT"
  ttl     = 300
  records = ["v=spf1 include:amazonses.com ~all"]
}

# --- DMARC record ---

resource "aws_route53_record" "dmarc" {
  zone_id = aws_route53_zone.parmail.zone_id
  name    = "_dmarc.parmail.${var.parent_domain}"
  type    = "TXT"
  ttl     = 300
  records = ["v=DMARC1; p=none; rua=mailto:${var.email_user}@parmail.${var.parent_domain}"]
}

output "domain_name" {
  value = "parmail.${var.parent_domain}"
}

output "email_address" {
  value = "${var.email_user}@parmail.${var.parent_domain}"
}
