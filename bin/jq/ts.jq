# Top senders by frequency
[
  .[] |
  .mail_pieces[] |
  select(.from_address.status == "resolved") |
  .from_address.address.name // "unknown"
] |
group_by(.) |
map({
  name: .[0],
  count: length
}) |
sort_by(-.count) |
["count,name"] +
[.[] | [.count, .name] | @csv] |
.[]
