include "shared";
def describe: "Mail pieces received per day";
def execute:
  group_by(.model_id) |
  map(
    .[0].model_id as $model |
    group_by(.received_date) |
    map({
      model: $model,
      day: .[0].received_date,
      pieces: ([.[] | .mail_pieces] | flatten | length)
    }) |
    sort_by(.day)
  ) | flatten;
