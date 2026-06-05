include "shared";
def describe: "Sender frequency with full address breakdown";
def execute:
  map(.mail_pieces) | flatten |
  map({
    name: .from_address?.name,
    street: .from_address?.street,
    city: .from_address?.city,
    state: .from_address?.state,
    zip: .from_address?.zip
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
