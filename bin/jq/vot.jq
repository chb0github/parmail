# Mail pieces received per day
group_by(.received_date) |
map({
  day: .[0].received_date,
  pieces: ([.[] | .mail_pieces] | flatten | length)
}) |
sort_by(.day) |
["day,pieces"] +
[.[] | [.day, .pieces] | @csv] |
.[]
