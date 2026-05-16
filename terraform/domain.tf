resource "random_pet" "domain" {
  length    = 2
  separator = ""
}

resource "aws_route53domains_registered_domain" "parmail" {
  domain_name = "${random_pet.domain.id}.click"

  name_server {
    name = aws_route53_zone.parmail.name_servers[0]
  }
  name_server {
    name = aws_route53_zone.parmail.name_servers[1]
  }
  name_server {
    name = aws_route53_zone.parmail.name_servers[2]
  }
  name_server {
    name = aws_route53_zone.parmail.name_servers[3]
  }
}

resource "aws_route53_zone" "parmail" {
  name = "${random_pet.domain.id}.click"
}

resource "aws_route53_record" "mx" {
  zone_id = aws_route53_zone.parmail.zone_id
  name    = "${random_pet.domain.id}.click"
  type    = "MX"
  ttl     = 300
  records = ["10 inbound-smtp.${var.aws_region}.amazonaws.com"]
}

resource "aws_ses_domain_identity" "parmail" {
  domain = "${random_pet.domain.id}.click"
}

resource "aws_route53_record" "ses_verification" {
  zone_id = aws_route53_zone.parmail.zone_id
  name    = "_amazonses.${random_pet.domain.id}.click"
  type    = "TXT"
  ttl     = 300
  records = [aws_ses_domain_identity.parmail.verification_token]
}

resource "aws_ses_domain_identity_verification" "parmail" {
  domain     = aws_ses_domain_identity.parmail.id
  depends_on = [aws_route53_record.ses_verification]
}

output "domain_name" {
  value = "${random_pet.domain.id}.click"
}

output "email_address" {
  value = "cbongiorno@${random_pet.domain.id}.click"
}
