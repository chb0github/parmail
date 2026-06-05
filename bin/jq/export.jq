include "shared";
def describe: "Export all manifests to flat CSV (one row per mail piece)";
def execute:
  group_by(.model_id) |
  map(
    .[0].model_id as $model |
    map(. as $email |
      $email.mail_pieces[] |
      {
        model: $model,
        email_id: $email.id,
        source_file: $email.source_file,
        received_date: $email.received_date,
        email_subject: $email.email_subject,
        email_message_id: $email.email_message_id,
        to_name: (.to_address.name // ""),
        to_street: (.to_address.street // ""),
        to_city: (.to_address.city // ""),
        to_state: (.to_address.state // ""),
        to_zip: (.to_address.zip // ""),
        to_resolved: .to_address.resolved,
        piece_id: .id,
        from_name: (.from_address.name // ""),
        from_street: (.from_address.street // ""),
        from_city: (.from_address.city // ""),
        from_state: (.from_address.state // ""),
        from_zip: (.from_address.zip // ""),
        from_resolved: .from_address.resolved,
        mail_type: .mail_type,
        confidence: .confidence,
        postmark_date: (.postmark_date // ""),
        mailer_text: (.mailer.full_text // ""),
        content_text: (.content.full_text // "")
      }
    )
  ) | flatten;
