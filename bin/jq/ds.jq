[.[] |
  .received_date as $date |
  .mail_pieces[] |
  select(.from_address.status == "resolved") |
  {
    name: (.from_address.address.name // "unknown"),
    city: (.from_address.address.city // ""),
    state: (.from_address.address.state // ""),
    mail_type: .mail_type,
    date: $date
  }
] |
group_by([.name, .city, .state, .mail_type]) |
map(select(length >= 5) | {
  count: length,
  name: .[0].name,
  city: .[0].city,
  state: .[0].state,
  mail_type: .[0].mail_type,
  first_seen: (map(.date) | sort | first),
  last_seen: (map(.date) | sort | last)
}) |
sort_by(-.count) |
["count,name,city,state,mail_type,first_seen,last_seen"] +
[.[] | [.count, .name, .city, .state, .mail_type, .first_seen, .last_seen] | @csv] |
.[]
