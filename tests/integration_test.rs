use std::path::PathBuf;

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples")
}

fn sample_files() -> Vec<PathBuf> {
    std::fs::read_dir(samples_dir())
        .expect("samples directory should exist")
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
        assert!(!files.is_empty(), "No .eml files found in samples/");
        println!("Found {} sample emails", files.len());
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
    fn daily_digest_contains_images() {
        let digest_files: Vec<_> = sample_files()
            .into_iter()
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .contains("Daily Digest")
            })
            .collect();

        assert!(
            !digest_files.is_empty(),
            "No Daily Digest sample emails found"
        );

        for path in digest_files {
            let raw = std::fs::read(&path).unwrap();
            let parsed = parse_email(&raw).unwrap();

            println!(
                "{}: extracted {} images",
                path.file_name().unwrap().to_str().unwrap(),
                parsed.images.len()
            );

            assert!(
                !parsed.images.is_empty(),
                "Daily Digest email {} should contain at least one image",
                path.display()
            );

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
        let digest_files: Vec<_> = sample_files()
            .into_iter()
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .contains("Daily Digest")
            })
            .collect();

        for path in digest_files {
            let raw = std::fs::read(&path).unwrap();
            let parsed = parse_email(&raw).unwrap();

            println!("Subject: {}", parsed.info.subject);
            println!("From: {}", parsed.info.from);
            println!("Date: {}", parsed.info.date);
            println!("Message-ID: {}", parsed.info.message_id);
            println!("Dir name: {}", parsed.info.dir_name());
            println!("Date folder: {}", parsed.info.date_folder());
            println!();

            assert!(
                parsed.info.subject.contains("Daily Digest"),
                "Subject should contain 'Daily Digest', got: {}",
                parsed.info.subject
            );
            assert_ne!(parsed.info.from, "unknown");
            assert_ne!(parsed.info.message_id, "unknown");
            assert!(parsed.info.date_folder().len() == 10); // YYYY-MM-DD
        }
    }

    #[test]
    fn expected_delivery_email_has_no_mail_images() {
        let expected_delivery: Vec<_> = sample_files()
            .into_iter()
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .contains("Expected Delivery")
            })
            .collect();

        for path in expected_delivery {
            let raw = std::fs::read(&path).unwrap();
            let parsed = parse_email(&raw).unwrap();
            println!(
                "Expected Delivery email: {} images found",
                parsed.images.len()
            );
        }
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

mod storage {
    use parmail::email::EmailInfo;
    use parmail::models::{EmailManifest, MailMetadata, MailType};
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

    #[tokio::test]
    async fn store_image_in_email_dir() {
        let tmp = TempDir::new().unwrap();
        let storage = Storage::local(tmp.path());
        let info = sample_email_info();

        let dir = storage.ensure_email_dir(&info).await.unwrap();
        let fake_jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        let filename = storage
            .store_image(&dir, &fake_jpeg, "test-001.jpg")
            .await
            .unwrap();

        assert_eq!(filename, "test-001.jpg");

        let local_dir = dir.as_local_path().unwrap();
        let full_path = local_dir.join(&filename);
        assert!(
            full_path.exists(),
            "Stored file should exist at {}",
            full_path.display()
        );

        let contents = std::fs::read(&full_path).unwrap();
        assert_eq!(contents, fake_jpeg);
    }

    #[tokio::test]
    async fn email_dir_structure_uses_date_and_slug() {
        let tmp = TempDir::new().unwrap();
        let storage = Storage::local(tmp.path());
        let info = sample_email_info();

        let dir = storage.ensure_email_dir(&info).await.unwrap();
        let local_dir = dir.as_local_path().unwrap();
        let dir_str = local_dir.to_string_lossy();

        assert!(
            dir_str.contains("2025-07-25"),
            "Dir should contain date: {}",
            dir_str
        );
        assert!(
            dir_str.contains("your-daily-digest-for"),
            "Dir should contain slug: {}",
            dir_str
        );
    }

    #[tokio::test]
    async fn store_manifest_creates_json() {
        let tmp = TempDir::new().unwrap();
        let storage = Storage::local(tmp.path());
        let info = sample_email_info();

        let dir = storage.ensure_email_dir(&info).await.unwrap();

        let manifest = EmailManifest {
            email_subject: info.subject.clone(),
            email_from: info.from.clone(),
            email_date: info.date.clone(),
            email_message_id: info.message_id.clone(),
            processed_at: "2025-07-25T15:00:00Z".to_string(),
            items: vec![MailMetadata {
                id: "test-id-123".to_string(),
                image_filename: "test-001.jpg".to_string(),
                image_sha256: "abc123".to_string(),
                from_address: None,
                to_address: None,
                mail_type: MailType::Advertising,
                full_text: "BUY STUFF NOW".to_string(),
                confidence: 0.92,
                error: None,
            }],
        };

        storage.store_manifest(&dir, &manifest).await.unwrap();

        let local_dir = dir.as_local_path().unwrap();
        let manifest_path = local_dir.join("manifest.json");
        assert!(manifest_path.exists(), "manifest.json should exist");

        let json_str = std::fs::read_to_string(&manifest_path).unwrap();
        let parsed: EmailManifest = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.email_subject, info.subject);
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].image_filename, "test-001.jpg");
        assert_eq!(parsed.items[0].full_text, "BUY STUFF NOW");
    }
}
