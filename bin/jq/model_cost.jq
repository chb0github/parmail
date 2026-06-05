include "shared";
def describe: "Token usage, cost, and cost-per-resolved-address by model";
# Requires --argjson prices with {model: [input_price, output_price]}
def execute:
  group_by(.model_id) |
  map(
    .[0].model_id as $model |
    (map(.usage) | {in: (map(.input_tokens) | add // 0), out: (map(.output_tokens) | add // 0)}) as $tok |
    (map(.mail_pieces) | flatten | map(select(.from_address.resolved)) | length) as $from_resolved |
    ($prices[$model] // [1, 5]) as $p |
    ($tok.in / 1000000 * $p[0] + $tok.out / 1000000 * $p[1]) as $cost |
    {
      model: $model,
      input_tokens: $tok.in,
      output_tokens: $tok.out,
      "input_$/M": $p[0],
      "output_$/M": $p[1],
      total_cost: ($cost * 10000 | round / 10000),
      from_resolved: $from_resolved,
      cost_per_resolved: (if $from_resolved > 0 then ($cost / $from_resolved * 10000 | round / 10000) else null end)
    }
  ) |
  sort_by(.total_cost) | reverse;
