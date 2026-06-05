# Parmail Status - June 4, 2026

## Last Batch Run Summary

**Completed:** 21:41 PDT, June 4, 2026

### Processing Stats
- **Total Duration:** 547 seconds (~9 minutes)
- **Emails Processed:** 1,056
- **Mail Pieces:** 2,549
- **Concurrency:** 8 workers
- **Model:** us.anthropic.claude-haiku-4-5-20251001-v1:0

### Cost Analysis
- **Total Cost:** $7.73
- **Input Tokens:** 4,277,916 ($3.42)
- **Output Tokens:** 1,076,834 ($4.31)
- **Cost per Resolved Address:** $0.0039

### Quality Metrics
- **Parse Success:** 100% (2,549/2,549 pieces)
- **From Address Resolved:** 0% (0/2,549) - known Haiku 4.5 issue
- **Mail Type Classified:** 98.5% (2,512/2,549)
- **Errors:** 1 API retry failure

### Architecture Changes (June 4, 2026)

**Output Structure Restructured:**
- Old: `results/{model}/{email_hash}/`
- New: `{email_id}/{piece_id}/{mailer,content}.jpg`
- Manifest now includes `image` field with relative paths
- Single manifest per email at root level

**Code Changes:**
- Schema v2: Flattened addresses, added `resolved` boolean
- Removed `AddressField` wrapper and `AddressStatus` enum
- Moved `to_address` from EmailManifest to MailPiece level
- Added piece-level subdirectories for images

**Deployment:**
- Lambda updated with new structure (ae43135)
- All 1,056 emails reprocessed with new format
- Results uploaded to s3://parmail-692140489268/output/
- Old format manifests cleared from S3

### Known Issues

1. **Recipient Address Extraction:** Haiku 4.5 extracts 0% recipient addresses (100% failure rate)
   - Sender addresses extract correctly (from_address)
   - Root cause: Possibly over-aggressive redaction interpretation
   - Next steps: Test with Sonnet 4.6 or Opus 4

2. **Build Environment:** Custom `cc` wrapper in `~/.local/bin` conflicts with Rust compiler
   - Workaround: `PATH="/usr/bin:/bin:$PATH" cargo build --release`
   - Documented in CLAUDE.md

### Next Steps

1. Test recipient extraction with higher-tier models (Sonnet 4.6/Opus 4)
2. Consider cost vs. quality tradeoffs for production model selection
3. Monitor for partial failures (currently 1 API retry out of 2,549 pieces)
4. Validate new output structure with frontend/display requirements

### Reports Generated

All reports updated in `reports/` directory:
- addresses.csv - sender frequency (recipients all null)
- agreement.csv - model agreement metrics  
- dq.csv - data quality breakdown
- ds.csv - data source statistics
- errors.csv - per-model error tracking
- export.csv - full data export (14MB+)
- model_cost.csv - cost analysis
- model_stats.csv - performance metrics
- mtb.csv - mail-to-business mapping
- ts.csv - time series analysis
- vot.csv - volume over time
