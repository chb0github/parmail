# Address resolution rates (resolved/redacted/unreadable/not_analyzed)
(
  [.[] | {field: "to", status: .to_address.status}] +
  [.[] | .mail_pieces[] | {field: "from", status: .from_address.status}]
) |
group_by([.field, .status]) |
map({
  field: .[0].field,
  status: .[0].status,
  count: length
}) |
sort_by([.field, -.count]) |
["field,status,count"] +
[.[] | [.field, .status, .count] | @csv] |
.[]
