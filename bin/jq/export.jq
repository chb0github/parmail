include "shared";
def describe: "Export all manifests to flat CSV (one row per mail piece)";
def execute:
  group_by(.model_id) |
  map(
    .[0].model_id as $model |
    .[] |
    .id as $eid |
    .source_file as $src |
    .received_date as $date |
    .email_subject as $subj |
    .email_message_id as $mid |
    .to_address as $to |
    .mail_pieces[] |
    {
      model: $model,
      email_id: $eid,
      source_file: $src,
      received_date: $date,
      email_subject: $subj,
      email_message_id: $mid,
      to_name: ($to.address.name // ""),
      to_street: ($to.address.street // ""),
      to_city: ($to.address.city // ""),
      to_state: ($to.address.state // ""),
      to_zip: ($to.address.zip // ""),
      to_status: $to.status,
      piece_id: .id,
      from_name: (.from_address.address.name // ""),
      from_street: (.from_address.address.street // ""),
      from_city: (.from_address.address.city // ""),
      from_state: (.from_address.address.state // ""),
      from_zip: (.from_address.address.zip // ""),
      from_status: .from_address.status,
      mail_type: .mail_type,
      confidence: .confidence,
      postmark_date: (.postmark_date // ""),
      mailer_filename: (.mailer.filename // ""),
      mailer_text: (.mailer.full_text // ""),
      content_filename: (.content.filename // ""),
      content_text: (.content.full_text // "")
    }
  ) | flatten;
