def to_csv:
  [.] | flatten(1) |
  select((. | length) > 0) // halt |
  (first | keys_unsorted) as $keys |
  ([$keys] + map([.[ $keys[] ]])) [] | @csv;

def pct(n; d): (n / ([d, 1] | max) * 1000 | round / 10);

def upcase: if . == null then null else ascii_upcase end;

def remove_formalities:
  if . == null then null
  else
    # Remove common titles and formalities
    gsub("\\bMR\\.?\\s*(AND\\s*MRS\\.?)?\\s*"; ""; "i") |
    gsub("\\bMRS\\.?\\s*"; ""; "i") |
    gsub("\\bMS\\.?\\s*"; ""; "i") |
    gsub("\\bMISS\\s*"; ""; "i") |
    gsub("\\bDR\\.?\\s*"; ""; "i") |
    gsub("\\bPROF\\.?\\s*"; ""; "i") |
    gsub("\\bREV\\.?\\s*"; ""; "i") |
    # Remove extra whitespace
    gsub("\\s+"; " ") |
    # Trim leading/trailing whitespace
    gsub("^\\s+|\\s+$"; "")
  end;

def remove_punctuation:
  if . == null then null
  else
    # Remove periods, commas
    gsub("\\."; "") |
    gsub(","; "") |
    # Clean up extra whitespace
    gsub("\\s+"; " ") |
    # Trim leading/trailing whitespace
    gsub("^\\s+|\\s+$"; "")
  end;

def normalize_street:
  if . == null then null
  else
    upcase |
    remove_punctuation |
    # Normalize directional suffixes (SE, NE, SW, NW must come after street type)
    # Fix: "DEVON SE ST" -> "DEVON ST SE"
    gsub("(?<street>.*?)\\s+(?<dir>SE|NE|SW|NW)\\s+(?<type>ST|AVE|DR|RD|LN|CT|PL|WAY|BLVD|TER|CIR)(?<rest>.*)"; "\(.street) \(.type) \(.dir)\(.rest)") |
    # Normalize street type abbreviations
    gsub("\\bSTREET\\b"; "ST") |
    gsub("\\bAVENUE\\b"; "AVE") |
    gsub("\\bDRIVE\\b"; "DR") |
    gsub("\\bROAD\\b"; "RD") |
    gsub("\\bLANE\\b"; "LN") |
    gsub("\\bCOURT\\b"; "CT") |
    gsub("\\bPLACE\\b"; "PL") |
    gsub("\\bBOULEVARD\\b"; "BLVD") |
    gsub("\\bTERRACE\\b"; "TER") |
    gsub("\\bCIRCLE\\b"; "CIR") |
    # Remove extra whitespace
    gsub("\\s+"; " ") |
    gsub("^\\s+|\\s+$"; "")
  end;

def normalize_zip:
  if . == null then null
  elif . == "" then null
  else
    # Strip +4 extension for grouping: "32909-9207" -> "32909"
    gsub("-.*$"; "")
  end;

def normalize_address:
  if . == null then null
  else {
    name: (.name | upcase | remove_formalities | remove_punctuation),
    street: (.street | normalize_street),
    city: (.city | upcase),
    state: (.state | upcase),
    zip: (.zip | normalize_zip),
    resolved: .resolved
  }
  end;
