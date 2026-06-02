include "shared";
def describe: "Top senders by frequency";
def execute:
  group_by(.model_id) |
  map(
    .[0].model_id as $model |
    [.[] | .mail_pieces[] |
      select(.from_address.status == "resolved") |
      .from_address.address.name // "unknown"
    ] |
    group_by(.) |
    map({
      model: $model,
      name: .[0],
      count: length
    }) |
    sort_by(-.count)
  ) | flatten;
