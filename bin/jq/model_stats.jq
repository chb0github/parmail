include "shared";
def describe: "Per-model parse and resolution rates";
def execute:
  group_by(.model_id) |
  map(
    .[0].model_id as $model |
    map(.mail_pieces) | flatten |
    length as $total |
    {
      model: $model,
      pieces: $total,
      parse_pct: pct($total - (map(select(.mailer.error != null or .content.error != null)) | length); $total),
      from_pct: pct(map(select(.from_address.status == "resolved")) | length; $total),
      type_pct: pct(map(select(.mail_type != "unknown" and .mail_type != null and .mail_type != "")) | length; $total)
    }
  ) |
  sort_by(-.parse_pct);
