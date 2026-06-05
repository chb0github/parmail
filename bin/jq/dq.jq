include "shared";
def describe: "Address resolution rates (resolved/null)";
def execute:
  group_by(.model_id) |
  map(
    .[0].model_id as $model |
    map(.mail_pieces) | flatten |
    map(
      {field: "from", resolved: .from_address.resolved},
      {field: "to", resolved: .to_address.resolved}
    ) |
    group_by([.field, .resolved]) |
    map({
      model: $model,
      field: .[0].field,
      resolved: .[0].resolved,
      count: length
    })
  ) | flatten | sort_by([.model, .field, -.count]);
