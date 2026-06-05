use std::path::PathBuf;

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../emails")
}

fn sample_files() -> Vec<PathBuf> {
    std::fs::read_dir(samples_dir())
        .expect("emails directory should exist")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("eml") {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

mod email_parsing {
    use super::*;
    use parmail::email::parse_email;

    #[test]
    fn samples_directory_exists_and_has_eml_files() {
        let files = sample_files();
        assert!(!files.is_empty(), "No .eml files found in ../emails/ - run bin/generate_test_emails.py");
        println!("Found {} sample emails", files.len());
        assert!(files.len() >= 3, "Expected at least 3 test emails (mailer_and_content, mailer_only, no_images)");
    }

    #[test]
    fn parse_all_samples_without_panic() {
        for path in sample_files() {
            let raw = std::fs::read(&path).unwrap();
            let result = parse_email(&raw);
            assert!(
                result.is_ok(),
                "Failed to parse {}: {:?}",
                path.display(),
                result.err()
            );
        }
    }

    #[test]
    fn mailer_and_content_email_has_both_images() {
        let path = samples_dir().join("mailer_and_content.eml");
        assert!(path.exists(), "mailer_and_content.eml should exist");

        let raw = std::fs::read(&path).unwrap();
        let parsed = parse_email(&raw).unwrap();

        println!("mailer_and_content.eml: extracted {} images", parsed.images.len());
        assert_eq!(parsed.images.len(), 2, "Should have exactly 2 images (mailer + content)");

        for img in &parsed.images {
            assert!(
                img.content_type.starts_with("image/"),
                "Expected image content type, got: {}",
                img.content_type
            );
            assert!(!img.data.is_empty(), "Image {} has empty data", img.filename);
            assert!(!img.filename.is_empty(), "Image should have a filename");
        }
    }

    #[test]
    fn mailer_only_email_has_one_image() {
        let path = samples_dir().join("mailer_only.eml");
        assert!(path.exists(), "mailer_only.eml should exist");

        let raw = std::fs::read(&path).unwrap();
        let parsed = parse_email(&raw).unwrap();

        println!("mailer_only.eml: extracted {} images", parsed.images.len());
        assert_eq!(parsed.images.len(), 1, "Should have exactly 1 image (mailer only)");
    }

    #[test]
    fn no_images_email_has_zero_images() {
        let path = samples_dir().join("no_images.eml");
        assert!(path.exists(), "no_images.eml should exist");

        let raw = std::fs::read(&path).unwrap();
        let parsed = parse_email(&raw).unwrap();

        println!("no_images.eml: extracted {} images", parsed.images.len());
        assert_eq!(parsed.images.len(), 0, "Should have 0 images");
    }

    #[test]
    fn extracted_images_have_data() {
        for path in sample_files() {
            let raw = std::fs::read(&path).unwrap();
            let parsed = parse_email(&raw).unwrap();

            for img in &parsed.images {
                assert!(!img.data.is_empty(), "Image {} has empty data", img.filename);
                assert!(
                    img.content_type.starts_with("image/"),
                    "Expected image content type, got: {}",
                    img.content_type
                );
            }
        }
    }

    #[test]
    fn email_info_extracted_correctly() {
        let path = samples_dir().join("mailer_and_content.eml");
        let raw = std::fs::read(&path).unwrap();
        let parsed = parse_email(&raw).unwrap();

        println!("Subject: {}", parsed.info.subject);
        println!("From: {}", parsed.info.from);
        println!("Date: {}", parsed.info.date);
        println!("Message-ID: {}", parsed.info.message_id);
        println!("ID: {}", parsed.info.id());
        println!("Date folder: {}", parsed.info.date_folder());

        assert!(parsed.info.subject.contains("Daily Digest"));
        assert!(parsed.info.from.contains("USPS"));
        assert_ne!(parsed.info.message_id, "unknown");
        assert_eq!(parsed.info.date_folder().len(), 10); // YYYY-MM-DD
    }


    #[test]
    fn image_filenames_are_non_empty() {
        for path in sample_files() {
            let raw = std::fs::read(&path).unwrap();
            let parsed = parse_email(&raw).unwrap();

            for img in &parsed.images {
                assert!(!img.filename.is_empty(), "Image should have a filename");
            }
        }
    }
}

mod s3_event_parsing {
    use parmail::models::S3Event;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    #[test]
    fn deserialize_single_record_event() {
        let json = std::fs::read_to_string(fixtures_dir().join("s3_event_single.json")).unwrap();
        let event: S3Event = serde_json::from_str(&json).unwrap();

        assert_eq!(event.records.len(), 1);
        assert_eq!(event.records[0].s3.bucket.name, "parmail-692140489268");
        assert_eq!(
            event.records[0].s3.object.key,
            "emails/03gk38n2o2i55pmbco4dl66kaiqhfic376bgaeo1"
        );
    }

    #[test]
    fn deserialize_multiple_record_event() {
        let json = std::fs::read_to_string(fixtures_dir().join("s3_event_multiple.json")).unwrap();
        let event: S3Event = serde_json::from_str(&json).unwrap();

        assert_eq!(event.records.len(), 2);
        assert_eq!(event.records[0].s3.object.key, "emails/first-email-key");
        assert_eq!(event.records[1].s3.object.key, "emails/second-email-key");
    }

    #[test]
    fn extra_fields_are_ignored() {
        let json = std::fs::read_to_string(fixtures_dir().join("s3_event_single.json")).unwrap();
        let event: S3Event = serde_json::from_str(&json).unwrap();
        assert_eq!(event.records.len(), 1);
    }
}

mod storage {
    use parmail::email::EmailInfo;
    use chrono::NaiveDate;
    use parmail::models::{Address, ContentHash, EmailManifest, MailImage, MailPiece, TokenUsage};
    use parmail::storage::Storage;
    use tempfile::TempDir;

    fn sample_email_info() -> EmailInfo {
        EmailInfo {
            subject: "Your Daily Digest for Fri, 7/25 is ready to view".to_string(),
            from: "USPS Informed Delivery".to_string(),
            date: "2025-07-25T14:24:32Z".to_string(),
            message_id: "20250725142432.612fb5af682bdb12@email.informeddelivery.usps.com"
                .to_string(),
        }
    }

    fn sample_manifest(info: &EmailInfo) -> EmailManifest {
        EmailManifest {
            id: info.id(),
            model_id: "us.anthropic.claude-haiku-4-5-20251001-v1:0".to_string(),
            source_file: "test.eml".to_string(),
            email_subject: info.subject.clone(),
            email_from: info.from.clone(),
            email_date: info.date.clone(),
            received_date: NaiveDate::from_ymd_opt(2025, 7, 25).unwrap(),
            email_message_id: info.message_id.clone(),
            processed_at: "2025-07-25T15:00:00Z".to_string(),
            mail_pieces: vec![MailPiece {
                id: "test-id-123".to_string(),
                from_address: None,
                to_address: None,
                mail_type: "advertising".to_string(),
                confidence: 0.92,
                postmark_date: None,
                mailer: Some(MailImage {
                    hash: ContentHash {
                        value: "abc123".to_string(),
                        hash_type: "xxh3".to_string(),
                    },
                    image: "test-id-123/mailer.jpg".to_string(),
                    full_text: "BUY STUFF NOW".to_string(),
                    error: None,
                }),
                content: None,
            }],
            usage: TokenUsage { input_tokens: 100, output_tokens: 50 },
        }
    }

    #[tokio::test]
    async fn store_image_in_piece_dir() {
        let tmp = TempDir::new().unwrap();
        let storage = Storage::local(tmp.path());
        let info = sample_email_info();

        let email_dir = storage.ensure_email_dir(&info).await.unwrap();
        let piece_dir = storage.ensure_piece_dir(&email_dir, "test-piece-123").await.unwrap();

        let fake_jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        let image_path = storage
            .store_image(&piece_dir, "test-piece-123", &fake_jpeg, "mailer.jpg")
            .await
            .unwrap();

        assert_eq!(image_path, "test-piece-123/mailer.jpg");
    }

    #[tokio::test]
    async fn email_dir_uses_content_hash() {
        let tmp = TempDir::new().unwrap();
        let storage = Storage::local(tmp.path());
        let info = sample_email_info();

        let dir = storage.ensure_email_dir(&info).await.unwrap();
        let local_dir = dir.as_local_path().unwrap();
        let dir_name = local_dir.file_name().unwrap().to_str().unwrap();

        assert_eq!(dir_name, info.id());
        assert_eq!(dir_name.len(), 16);
    }

    #[tokio::test]
    async fn store_manifest_creates_json() {
        let tmp = TempDir::new().unwrap();
        let storage = Storage::local(tmp.path());
        let info = sample_email_info();

        let dir = storage.ensure_email_dir(&info).await.unwrap();

        let manifest = sample_manifest(&info);

        storage.store_manifest(&dir, &manifest).await.unwrap();

        let local_dir = dir.as_local_path().unwrap();
        let manifest_path = local_dir.join("manifest.json");
        assert!(manifest_path.exists(), "manifest.json should exist");

        let json_str = std::fs::read_to_string(&manifest_path).unwrap();
        let parsed: EmailManifest = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.email_subject, info.subject);
        assert_eq!(parsed.mail_pieces.len(), 1);
        let mailer = parsed.mail_pieces[0].mailer.as_ref().unwrap();
        assert_eq!(mailer.image, "test-id-123/mailer.jpg");
        assert_eq!(mailer.full_text, "BUY STUFF NOW");
    }

    #[tokio::test]
    async fn load_valid_manifest_returns_some_when_no_errors() {
        let tmp = TempDir::new().unwrap();
        let storage = Storage::local(tmp.path());
        let info = sample_email_info();

        let dir = storage.ensure_email_dir(&info).await.unwrap();
        let manifest = sample_manifest(&info);
        storage.store_manifest(&dir, &manifest).await.unwrap();

        let loaded = storage.load_valid_manifest(&info).await;
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, info.id());
    }

    #[tokio::test]
    async fn load_valid_manifest_returns_none_when_errors() {
        let tmp = TempDir::new().unwrap();
        let storage = Storage::local(tmp.path());
        let info = sample_email_info();

        let dir = storage.ensure_email_dir(&info).await.unwrap();
        let mut manifest = sample_manifest(&info);
        manifest.mail_pieces[0].mailer.as_mut().unwrap().error =
            Some("Bedrock converse API call failed after retries".to_string());
        storage.store_manifest(&dir, &manifest).await.unwrap();

        let loaded = storage.load_valid_manifest(&info).await;
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn load_valid_manifest_returns_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        let storage = Storage::local(tmp.path());
        let info = sample_email_info();

        let loaded = storage.load_valid_manifest(&info).await;
        assert!(loaded.is_none());
    }
}
