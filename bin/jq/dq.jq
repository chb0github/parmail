include "shared";
def describe: "Address resolution rates (resolved/redacted/unreadable/not_analyzed)";
def execute:
  group_by(.model_id) |
  map(
    .[0].model_id as $model |
    (
      [.[] | {field: "to", status: .to_address.status}] +
      [.[] | .mail_pieces[] | {field: "from", status: .from_address.status}]
    ) |
    group_by([.field, .status]) |
    map({
      model: $model,
      field: .[0].field,
      status: .[0].status,
      count: length
    })
  ) | flatten | sort_by([.model, .field, -.count]);
