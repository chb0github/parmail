def to_csv:
  [.] | flatten(1) |
  select((. | length) > 0) // halt |
  (first | keys_unsorted) as $keys |
  ([$keys] + map([.[ $keys[] ]])) [] | @csv;

def pct(n; d): (n / ([d, 1] | max) * 1000 | round / 10);
