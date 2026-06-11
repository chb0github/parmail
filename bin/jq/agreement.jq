include "shared";
def describe: "Cross-model agreement rates on from_name, street, city, mail_type, to_name";
def agree(f):
  [.[] | [.[].data | f // "" | select(. != "")] | unique | select(length <= 1)] | length;

def execute:
  [.[] | .model_id as $model | {model: $model, id: .id, data: .}] |
  group_by(.id) |
  map(select(length > 1)) |
  length as $total |
  [
    {field: "from_name",   agree: agree(.mail_pieces[0].from_address | normalize_address | .name),   total: $total},
    {field: "from_street", agree: agree(.mail_pieces[0].from_address | normalize_address | .street), total: $total},
    {field: "from_city",   agree: agree(.mail_pieces[0].from_address | normalize_address | .city),   total: $total},
    {field: "mail_type",   agree: agree(.mail_pieces[0].mail_type),                                  total: $total},
    {field: "to_name",     agree: agree(.mail_pieces[0].to_address | normalize_address | .name),     total: $total}
  ] |
  map(. + {pct: (.agree / .total * 1000 | round / 10)});
