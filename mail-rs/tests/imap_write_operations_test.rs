//! Integration tests for IMAP write operations (STORE, COPY, EXPUNGE)

use mail_rs::imap::{Mailbox, StoreOperation};
use std::fs;
use tempfile::TempDir;

/// Helper to create a test maildir structure
fn setup_test_maildir() -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let email = "test@example.com";
    let maildir = temp_dir.path().join(email);

    // Create maildir structure
    fs::create_dir_all(maildir.join("new")).unwrap();
    fs::create_dir_all(maildir.join("cur")).unwrap();
    fs::create_dir_all(maildir.join("tmp")).unwrap();

    // Create test emails in new/ (unread)
    fs::write(
        maildir.join("new/1.eml"),
        "From: alice@example.com\r\nSubject: Test 1\r\n\r\nBody 1"
    ).unwrap();

    fs::write(
        maildir.join("new/2.eml"),
        "From: bob@example.com\r\nSubject: Test 2\r\n\r\nBody 2"
    ).unwrap();

    // Create test email in cur/ (with flags)
    fs::write(
        maildir.join("cur/3.eml:2,S"),
        "From: charlie@example.com\r\nSubject: Test 3\r\n\r\nBody 3"
    ).unwrap();

    (temp_dir, email.to_string())
}

#[test]
fn test_store_add_seen_flag() {
    let (temp_dir, email) = setup_test_maildir();
    let mut mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();

    // Initially, message 1 should have no flags (in new/)
    let msg = mailbox.get_message(1).unwrap();
    assert!(msg.flags.is_empty());

    // Add \Seen flag to message 1
    let result = mailbox.store_flags("1", &StoreOperation::Add, &["\\Seen".to_string()]);
    assert!(result.is_ok());

    // Check in-memory that flag was added
    let msg = mailbox.get_message(1).unwrap();
    assert!(msg.flags.contains(&"\\Seen".to_string()));

    // Check that file was moved to cur/ with :2,S suffix
    let cur_dir = temp_dir.path().join(&email).join("cur");
    let entries: Vec<_> = fs::read_dir(&cur_dir).unwrap().collect();
    assert!(entries.len() > 0);

    let has_seen_flag = entries.iter().any(|e| {
        e.as_ref().unwrap().file_name().to_string_lossy().contains(":2,S")
    });
    assert!(has_seen_flag, "File should have :2,S suffix");

    // Reload mailbox to verify persistence
    let mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();

    // Find message with "Test 1" subject
    let msg_with_flag = mailbox.messages().iter().find(|m| {
        String::from_utf8_lossy(&m.content).contains("Test 1")
    }).expect("Should find Test 1 message");

    assert!(msg_with_flag.flags.contains(&"\\Seen".to_string()));
}

#[test]
fn test_store_add_multiple_flags() {
    let (temp_dir, email) = setup_test_maildir();
    let mut mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();

    // Add multiple flags at once
    let result = mailbox.store_flags(
        "1",
        &StoreOperation::Add,
        &["\\Seen".to_string(), "\\Flagged".to_string()]
    );
    assert!(result.is_ok());

    // Check in-memory
    let msg = mailbox.get_message(1).unwrap();
    assert!(msg.flags.contains(&"\\Seen".to_string()));
    assert!(msg.flags.contains(&"\\Flagged".to_string()));

    // Check filename has both flags (FS in alphabetical order)
    let cur_dir = temp_dir.path().join(&email).join("cur");
    let entries: Vec<_> = fs::read_dir(&cur_dir).unwrap().collect();
    let has_both_flags = entries.iter().any(|e| {
        let name = e.as_ref().unwrap().file_name().to_string_lossy().to_string();
        name.contains(":2,") && name.contains("F") && name.contains("S")
    });
    assert!(has_both_flags, "File should have both F and S flags");

    // Reload and check persistence
    let mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();
    let msg_with_flags = mailbox.messages().iter().find(|m| {
        String::from_utf8_lossy(&m.content).contains("Test 1")
    }).expect("Should find Test 1 message");

    assert!(msg_with_flags.flags.contains(&"\\Seen".to_string()));
    assert!(msg_with_flags.flags.contains(&"\\Flagged".to_string()));
}

#[test]
fn test_store_remove_flag() {
    let (temp_dir, email) = setup_test_maildir();
    let mut mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();

    // First add a flag
    mailbox.store_flags("1", &StoreOperation::Add, &["\\Seen".to_string()]).unwrap();

    // Verify it was added
    let msg = mailbox.get_message(1).unwrap();
    assert!(msg.flags.contains(&"\\Seen".to_string()));

    // Now remove the flag
    mailbox.store_flags("1", &StoreOperation::Remove, &["\\Seen".to_string()]).unwrap();

    // Verify it was removed
    let msg = mailbox.get_message(1).unwrap();
    assert!(!msg.flags.contains(&"\\Seen".to_string()));
}

#[test]
fn test_store_replace_flags() {
    let (temp_dir, email) = setup_test_maildir();
    let mut mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();

    // Add some initial flags
    mailbox.store_flags(
        "1",
        &StoreOperation::Add,
        &["\\Seen".to_string(), "\\Flagged".to_string()]
    ).unwrap();

    // Replace with different flags
    mailbox.store_flags(
        "1",
        &StoreOperation::Replace,
        &["\\Deleted".to_string()]
    ).unwrap();

    // Verify only \Deleted flag remains
    let msg = mailbox.get_message(1).unwrap();
    assert_eq!(msg.flags.len(), 1);
    assert!(msg.flags.contains(&"\\Deleted".to_string()));
    assert!(!msg.flags.contains(&"\\Seen".to_string()));
    assert!(!msg.flags.contains(&"\\Flagged".to_string()));
}

#[test]
fn test_store_sequence_range() {
    let (temp_dir, email) = setup_test_maildir();
    let mut mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();

    // Add flag to range of messages: 1:2
    let result = mailbox.store_flags("1:2", &StoreOperation::Add, &["\\Seen".to_string()]);
    assert!(result.is_ok());

    // Check both messages have the flag
    let msg1 = mailbox.get_message(1).unwrap();
    let msg2 = mailbox.get_message(2).unwrap();

    assert!(msg1.flags.contains(&"\\Seen".to_string()));
    assert!(msg2.flags.contains(&"\\Seen".to_string()));
}

#[test]
fn test_expunge_deletes_marked_messages() {
    let (temp_dir, email) = setup_test_maildir();
    let mut mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();

    let initial_count = mailbox.message_count();
    assert_eq!(initial_count, 3);

    // Mark message 1 as deleted
    mailbox.store_flags("1", &StoreOperation::Add, &["\\Deleted".to_string()]).unwrap();

    // Expunge should remove it
    let expunged = mailbox.expunge().unwrap();
    assert_eq!(expunged.len(), 1);
    assert_eq!(expunged[0], 1);

    // Message count should decrease
    assert_eq!(mailbox.message_count(), 2);

    // Reload mailbox to verify file was actually deleted
    let mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();
    assert_eq!(mailbox.message_count(), 2);
}

#[test]
fn test_expunge_multiple_messages() {
    let (temp_dir, email) = setup_test_maildir();
    let mut mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();

    // Mark multiple messages as deleted
    mailbox.store_flags("1:2", &StoreOperation::Add, &["\\Deleted".to_string()]).unwrap();

    // Expunge should remove both
    let expunged = mailbox.expunge().unwrap();
    assert_eq!(expunged.len(), 2);

    // Only 1 message should remain
    assert_eq!(mailbox.message_count(), 1);
}

#[test]
fn test_expunge_renumbers_sequences() {
    let (temp_dir, email) = setup_test_maildir();
    let mut mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();

    // Mark message 2 as deleted (middle message)
    mailbox.store_flags("2", &StoreOperation::Add, &["\\Deleted".to_string()]).unwrap();

    // Expunge
    mailbox.expunge().unwrap();

    // Sequences should be renumbered: 1, 2 (was 1, 3)
    assert_eq!(mailbox.message_count(), 2);
    let msg1 = mailbox.get_message(1).unwrap();
    let msg2 = mailbox.get_message(2).unwrap();

    assert_eq!(msg1.sequence, 1);
    assert_eq!(msg2.sequence, 2);
}

#[test]
fn test_copy_message_to_sent() {
    let (temp_dir, email) = setup_test_maildir();
    let maildir = temp_dir.path().join(&email);

    // Create Sent folder structure
    let sent_folder = maildir.join(".Sent");
    fs::create_dir_all(sent_folder.join("new")).unwrap();
    fs::create_dir_all(sent_folder.join("cur")).unwrap();

    let mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();

    // Copy message 1 to Sent
    let result = mailbox.copy_messages("1", "Sent", &email, temp_dir.path());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);

    // Verify original still exists in INBOX
    assert_eq!(mailbox.message_count(), 3);

    // Verify copy exists in Sent
    let sent_mailbox = Mailbox::open(&email, "Sent", temp_dir.path()).unwrap();
    assert_eq!(sent_mailbox.message_count(), 1);
}

#[test]
fn test_copy_preserves_flags() {
    let (temp_dir, email) = setup_test_maildir();
    let maildir = temp_dir.path().join(&email);

    // Create destination folder
    let archive_folder = maildir.join(".Archive");
    fs::create_dir_all(archive_folder.join("new")).unwrap();
    fs::create_dir_all(archive_folder.join("cur")).unwrap();

    let mut mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();

    // Add flags to message 1
    mailbox.store_flags("1", &StoreOperation::Add, &["\\Seen".to_string(), "\\Flagged".to_string()]).unwrap();

    // Copy to Archive
    mailbox.copy_messages("1", "Archive", &email, temp_dir.path()).unwrap();

    // Check that flags were preserved in copy
    let archive_mailbox = Mailbox::open(&email, "Archive", temp_dir.path()).unwrap();
    let copied_msg = archive_mailbox.get_message(1).unwrap();

    assert!(copied_msg.flags.contains(&"\\Seen".to_string()));
    assert!(copied_msg.flags.contains(&"\\Flagged".to_string()));
}

#[test]
fn test_copy_multiple_messages() {
    let (temp_dir, email) = setup_test_maildir();
    let maildir = temp_dir.path().join(&email);

    // Create Trash folder
    let trash_folder = maildir.join(".Trash");
    fs::create_dir_all(trash_folder.join("new")).unwrap();

    let mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();

    // Copy multiple messages (1:2)
    let result = mailbox.copy_messages("1:2", "Trash", &email, temp_dir.path());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 2);

    // Verify copies exist
    let trash_mailbox = Mailbox::open(&email, "Trash", temp_dir.path()).unwrap();
    assert_eq!(trash_mailbox.message_count(), 2);
}

#[test]
fn test_full_workflow_mark_delete_expunge() {
    let (temp_dir, email) = setup_test_maildir();
    let mut mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();

    assert_eq!(mailbox.message_count(), 3);

    // 1. Mark message as seen
    mailbox.store_flags("1", &StoreOperation::Add, &["\\Seen".to_string()]).unwrap();

    // 2. Mark as deleted
    mailbox.store_flags("1", &StoreOperation::Add, &["\\Deleted".to_string()]).unwrap();

    // Verify it has both flags
    let msg = mailbox.get_message(1).unwrap();
    assert!(msg.flags.contains(&"\\Seen".to_string()));
    assert!(msg.flags.contains(&"\\Deleted".to_string()));

    // 3. Expunge to permanently remove
    let expunged = mailbox.expunge().unwrap();
    assert_eq!(expunged.len(), 1);

    // 4. Verify it's gone
    assert_eq!(mailbox.message_count(), 2);

    // 5. Reload from disk and verify persistence
    let mailbox = Mailbox::open(&email, "INBOX", temp_dir.path()).unwrap();
    assert_eq!(mailbox.message_count(), 2);
}
