include "shared";
def describe: "Recipient frequency with full address breakdown";
def execute:
  map(.mail_pieces) | flatten |
  map({
    name: .to_address?.name,
    street: .to_address?.street,
    city: .to_address?.city,
    state: .to_address?.state,
    zip: .to_address?.zip
  }) |
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
