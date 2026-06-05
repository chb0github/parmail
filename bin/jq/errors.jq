include "shared";
def describe: "Per-model error counts grouped by error message";
def execute:
  group_by(.model_id) |
  map(
    .[0].model_id as $model |
    map(.mail_pieces) | flatten |
    map(select(.mailer.error != null or (.content != null and .content.error != null))) |
    group_by(.mailer.error // .content.error) |
    map({
      model: $model,
      error: (.[0].mailer.error // .[0].content.error),
      count: length
    })
  ) | flatten | sort_by([.model, -.count]);
