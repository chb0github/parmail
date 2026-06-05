# Parmail

## Architecture

Inbound emails arrive via SES → SES stores raw email (no extension, message ID as key) in S3 (`emails/` prefix) → S3 notification triggers the Lambda → Lambda processes them and writes output to S3 (`output/` prefix).

**Important:** SES stores emails in S3 without file extensions. The processor accepts any file format and does not filter by extension.

### Unified Processing Pipeline

Lambda and CLI use the exact same code path (as of June 2026 refactoring):

```
EmailSource (Local | S3)
    ↓
fetch_email()           // unified input abstraction
    ↓
process_raw_email()     // core pipeline (model inference + analysis)
    ↓
Storage (Local | S3)    // unified output abstraction
```

**Key modules:**
- `input.rs`: `EmailSource` enum + `fetch_email()` - handles both local files and S3 objects
- `processor.rs`: `process_raw_email()` - core pipeline used by both Lambda and CLI
- `storage.rs`: `Storage` trait - writes manifests/images to local or S3
- `lambda.rs`: Thin wrapper that converts S3 events to `EmailSource::S3` and calls the core pipeline
- `main.rs`: CLI that converts paths to `EmailSource` and calls the core pipeline

**Helpers:**
- `email::get_images(&[ExtractedImage])` - extracts (mailer_bytes, content_bytes) from a mail piece

## AWS

All AWS CLI commands must use `--profile thetaone`.
Terraform commands must use `AWS_PROFILE=thetaone` prefix.
Parent domain: `thetaone.io` (set via `TF_VAR_parent_domain=thetaone.io` or `-var="parent_domain=thetaone.io"`).

## GitHub

This project uses the personal GitHub identity `chb0github`. Switch before any gh/git operations:
```bash
gh auth switch --user chb0github
```
Switch back when done.

## Rust/Cargo Builds

The user has a custom `cc` command wrapper in `~/.local/bin` that conflicts with the system compiler. All cargo commands must use system PATH:
```bash
PATH="/usr/bin:/bin:$PATH" cargo build --release
PATH="/usr/bin:/bin:$PATH" cargo test
```

## SMS Notifications

Send SMS updates via CallCentric (from 425-394-2504 to user's Google Fi 703-975-4376):
```bash
~/dev/mine/cc_sms/cc sms send -m "message text" 7039754376
```

## Operational Workflows

**Important:** Never commit the `results/` or `emails_retry/` directories to git. These contain generated output and local email files that should not be in version control.

### Full Reconciliation (Rare)

When emails have processing errors or S3 output/ needs to be rebuilt from scratch:

```bash
# 1. Delete all S3 manifests
aws s3 rm "s3://parmail-692140489268/output/" --recursive --profile thetaone

# 2. Sync emails from S3
aws s3 sync "s3://parmail-692140489268/emails/" emails/ --profile thetaone

# 3. Process errored emails (resume logic skips existing valid manifests)
./target/release/parmail process \
  --model "us.anthropic.claude-haiku-4-5-20251001-v1:0" \
  --storage-dir "results/claude-haiku-4-5-20251001-v1" \
  -c 2 emails_retry/

# 4. Push all results back to S3
aws s3 sync "results/claude-haiku-4-5-20251001-v1" "s3://parmail-692140489268/output/" --profile thetaone

# 5. Regenerate all reports
./bin/report.sh
```

**Note:** `bin/sync.sh --reconcile` does steps 2-4 automatically but processes ALL gaps, not just `emails_retry/`. The manual workflow above gives more control for reprocessing specific error cases.

### Error Recovery

- Errored emails are tracked in `emails_retry/` directory
- The parmail processor has resume support: it skips emails with valid `manifest.json` files
- Check `reports/errors.csv` after processing to identify model-specific failure patterns
- Most errors come from Nova Lite model; Haiku 4.5 is most reliable

**Reprocessing unparseable responses:**
```bash
./bin/reprocess_unparseable.sh
```
Finds all emails with "No parseable response from model" errors, deletes their manifests, and reprocesses with the new escape fix and disabled guardrails. Remaining unparseable responses are saved as `results/_unparseable/{model}/{email_id}.json` for easy correlation.

### Reports

Run all reports with `./bin/report.sh`. Available reports:
- `addresses.csv` - Address frequency with person names (all address fields)
- `agreement.csv` - Agreement metrics
- `dq.csv` - Data quality
- `ds.csv` - Data source statistics  
- `errors.csv` - Per-model error counts by message
- `export.csv` - Full data export (all fields, 14MB+)
- `model_cost.csv` - Cost analysis per model
- `model_stats.csv` - Model performance statistics
- `mtb.csv` - Mail-to-business mapping
- `ts.csv` - Time series analysis
- `vot.csv` - Volume over time

## Technical Details

### Manifest Schema (v2 - June 2026)

**Schema refactoring completed:**
- Address structure flattened: `from_address.address.name` → `from_address.name`
- Removed `AddressField` wrapper and `AddressStatus` enum
- Added `resolved: bool` field directly to `Address` (true if successfully extracted)
- Added `image: Option<String>` to `MailImage` (stores the analyzed image filename for easy correlation)
- Moved `to_address` from `EmailManifest` to `MailPiece` level (each piece has independent sender/recipient)

**Migration:**
- All existing manifests backported with `bin/migrate_manifests.sh`
- Reports updated to use flattened structure: `.from_address.name` instead of `.from_address.address.name`
- Status now tracked via `resolved: bool` instead of enum (`resolved`, `redacted`, `unreadable`, `not_analyzed`)

### Output Directory Structure (v2 - June 2026)

**Breaking change:** Output structure changed from `results/{model}/{hash}/` to `{email_id}/{piece_id}/` format.

**New structure:**
```
{email_id}/                          # xxh3_64 hash of email Message-ID
  manifest.json                      # contains all pieces for this email
  {piece_id_1}/                      # xxh3_64 hash of piece identifier
    mailer.jpg                       # exterior front
    content.jpg                      # exterior back (if exists)
  {piece_id_2}/
    mailer.jpg
    content.jpg
```

**Manifest structure:**
- Each `MailImage` has an `image` field with relative path from email directory
- Example: `"image": "046a09c35047f728/mailer.jpg"`
- Makes it trivial to display: `{email_id}/{piece_id}/mailer.jpg`

**Migration:**
- Old `results/` directories (organized by model) are incompatible with new format
- Production will only use one model, so structuring by model doesn't make sense
- New format prioritizes easy lookup by email ID

**Test results:**
- `results_new/` contains 10 sample emails processed with new structure (1.4MB)
- Validated: image paths, manifest structure, piece directories

### Email Hash Computation

Parmail uses `xxh3_64` (not MD5) to hash the email Message-ID and piece IDs for directory names:
- Email directory: `format!("{:016x}", xxh3_64(message_id.as_bytes()))`
- Piece directory: `format!("{:016x}", xxh3_64(piece_id.as_bytes()))`
- Example: Message-ID `335743615.263857.1634912068303.JavaMail.prmin@eagnmncom1b38.usps.gov` → hash `68612578126e984c`

### Resume Logic

When processing emails (`src/processor.rs:55-58`):
1. Parse the email to extract Message-ID
2. Compute xxh3_64 hash of Message-ID
3. Check if `{storage_dir}/{hash}/manifest.json` exists and is valid
4. If valid manifest exists: skip processing, return existing manifest (reports "OK" but creates nothing new)
5. If no valid manifest: process normally and create output

**Important:** "OK" in processing output means either successfully processed OR successfully skipped due to existing valid manifest. Check manifest timestamps to determine which occurred.

### Partial Failures

Manifests can be "valid" but contain individual image analysis errors:
- A manifest exists → resume logic skips the email
- Individual mail pieces inside may have `"error": "Bedrock converse API call failed after retries"`
- These partial failures result in `null` addresses and are counted in `reports/errors.csv`
- **Resume logic does NOT retry partial failures** - once a manifest exists, that email is considered processed

To retry partial failures:
1. Delete the specific manifest directory: `rm -rf results/{model}/{hash}/`
2. Reprocess that email: the processor will create a fresh manifest
3. For Haiku 4.5: ~15 emails have partial failures (transient API errors, not model quality issues)

### Unparseable Responses

When models return responses that can't be parsed as valid JSON:
- The error in manifest shows: `"No parseable response from model: <200-char snippet>"`
- **Full responses are saved** in `results/_unparseable/{model_id}/NNNNNN.json`
- These files contain the complete, unmodified raw text from the model
- Common causes breakdown (by fixability):

**Fixable (escape issues only):**
- Invalid JSON escape sequences like `\'` instead of `'` (Nova Pro: ~44% of failures)
- Contains usable data, just needs `raw_text.replace(r"\'", "'")`

**Unfixable (structural problems):**
- Model returns JSON schema definition instead of data (Nova Lite: 100% of failures)
- Content filter blocks: `" The generated text has been blocked by our content filters."`
- Truncated JSON: missing closing braces/brackets (hit token limit mid-response)

**Content Filtering:**
- Bedrock filters PII/addresses in analyzed mail images
- Disabled via `.set_guardrail_config(None)` in converse API call
- Previous runs had 5+ Nova Lite filter blocks; Claude models had 0

### Recipient Address Extraction Issue

**Known Quality Issue (June 2026):**
- Haiku 4.5 extracts 0 recipient addresses out of 20,943 mail pieces (100% failure rate)
- Sender addresses (`from_address`) extract correctly
- Recipient addresses are clearly visible in the scanned mail images (e.g., "Mr. and Mrs. John Artman, 1248 Devon Street SE, Palm Bay, FL 32909-9207")
- The analysis prompt explicitly instructs: "Extract the sender address, recipient address, classify the mail type..."
- Root cause unknown: possibly over-aggressive interpretation of the redaction instruction ("USPS often redacts the recipient address with a white rectangle"), schema prioritization issue, or model limitation
- **Impact:** `reports/recipients.csv` shows all nulls; only `reports/senders.csv` is usable
- **Next steps:** Test with Sonnet 4/4.6 or Opus 4 to see if higher-tier models extract recipients correctly
