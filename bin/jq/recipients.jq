include "shared";
def describe: "Recipient frequency with full address breakdown";
def execute:
  map(.mail_pieces) | flatten |
  map(.to_address | normalize_address) |
  group_by([.name, .street, .city, .state, .zip]) |
  map({
    name: .[0].name,
    street: .[0].street,
    city: .[0].city,
    state: .[0].state,
    zip: .[0].zip,
    count: length
  }) |
  sort_by(-.count);
