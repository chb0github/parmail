include "shared";
def describe: "Mail type breakdown (advertising/financial/personal/etc) with percentages";
def execute:
  group_by(.model_id) |
  map(
    .[0].model_id as $model |
    map(.mail_pieces) | flatten |
    map(.mail_type) |
    length as $total |
    group_by(.) |
    map({
      model: $model,
      mail_type: .[0],
      count: length,
      pct: pct(length; $total)
    }) |
    sort_by(-.count)
  ) | flatten;
