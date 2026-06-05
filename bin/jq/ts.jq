include "shared";
def describe: "Top senders by frequency";
def execute:
  group_by(.model_id) |
  map(
    .[0].model_id as $model |
    map(.mail_pieces) | flatten |
    map(select(.from_address.resolved) | .from_address.name // "unknown") |
    group_by(.) |
    map({
      model: $model,
      name: .[0],
      count: length
    }) |
    sort_by(-.count)
  ) | flatten;
