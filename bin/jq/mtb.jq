[.[] | .mail_pieces[].mail_type] |
length as $total |
group_by(.) |
map({
  mail_type: .[0],
  count: length,
  pct: (length / $total * 100 | . * 10 | round / 10)
}) |
sort_by(-.count) |
["mail_type,count,pct"] +
[.[] | [.mail_type, .count, .pct] | @csv] |
.[]
