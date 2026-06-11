include "shared";
def describe: "Export all manifests to flat CSV (one row per mail piece)";
def execute:
  group_by(.model_id) |
  map(
    .[0].model_id as $model |
    map(. as $email |
      $email.mail_pieces[] |
      (.to_address | normalize_address) as $to |
      (.from_address | normalize_address) as $from |
      {
        model: $model,
        email_id: $email.id,
        source_file: $email.source_file,
        received_date: $email.received_date,
        email_subject: $email.email_subject,
        email_message_id: $email.email_message_id,
        to_name: ($to.name // ""),
        to_street: ($to.street // ""),
        to_city: ($to.city // ""),
        to_state: ($to.state // ""),
        to_zip: ($to.zip // ""),
        to_resolved: $to.resolved,
        piece_id: .id,
        from_name: ($from.name // ""),
        from_street: ($from.street // ""),
        from_city: ($from.city // ""),
        from_state: ($from.state // ""),
        from_zip: ($from.zip // ""),
        from_resolved: $from.resolved,
        mail_type: .mail_type,
        confidence: .confidence,
        postmark_date: (.postmark_date // ""),
        mailer_text: (.mailer.full_text // ""),
        content_text: (.content.full_text // "")
      }
    )
  ) | flatten;
