use mail_rs::smtp::SmtpQueue;

#[tokio::test]
async fn test_queue_enqueue() {
    // Create in-memory SQLite database
    let queue = SmtpQueue::new("sqlite::memory:").await.unwrap();

    // Enqueue an email
    let id = queue
        .enqueue(
            "sender@example.com",
            "recipient@example.com",
            b"Subject: Test\r\n\r\nHello!",
        )
        .await
        .unwrap();

    assert!(!id.is_empty());
}

#[tokio::test]
async fn test_queue_get_pending() {
    let queue = SmtpQueue::new("sqlite::memory:").await.unwrap();

    // Enqueue multiple emails
    queue
        .enqueue("s1@example.com", "r1@example.com", b"Email 1")
        .await
        .unwrap();
    queue
        .enqueue("s2@example.com", "r2@example.com", b"Email 2")
        .await
        .unwrap();

    // Get pending emails
    let pending = queue.get_pending(10).await.unwrap();
    assert_eq!(pending.len(), 2);
}

#[tokio::test]
async fn test_queue_mark_sent() {
    let queue = SmtpQueue::new("sqlite::memory:").await.unwrap();

    let id = queue
        .enqueue("sender@example.com", "recipient@example.com", b"Test")
        .await
        .unwrap();

    // Mark as sent
    queue.mark_sent(&id).await.unwrap();

    // Should not appear in pending anymore
    let pending = queue.get_pending(10).await.unwrap();
    assert_eq!(pending.len(), 0);
}

#[tokio::test]
async fn test_queue_retry_logic() {
    let queue = SmtpQueue::new("sqlite::memory:").await.unwrap();

    let id = queue
        .enqueue("sender@example.com", "recipient@example.com", b"Test")
        .await
        .unwrap();

    // Mark as failed
    queue.mark_failed(&id, "Temporary error", 0).await.unwrap();

    // Should still exist but with retry scheduled
    let pending = queue.get_pending(10).await.unwrap();
    // Will be empty because next_retry_at is in the future
    assert_eq!(pending.len(), 0);
}

#[tokio::test]
async fn test_queue_max_retries() {
    let queue = SmtpQueue::new("sqlite::memory:").await.unwrap();

    let id = queue
        .enqueue("sender@example.com", "recipient@example.com", b"Test")
        .await
        .unwrap();

    // Fail 5 times (max retries)
    queue.mark_failed(&id, "Error 1", 0).await.unwrap();
    queue.mark_failed(&id, "Error 2", 1).await.unwrap();
    queue.mark_failed(&id, "Error 3", 2).await.unwrap();
    queue.mark_failed(&id, "Error 4", 3).await.unwrap();
    queue.mark_failed(&id, "Error 5", 4).await.unwrap(); // Should bounce now

    // Verify it's not in pending (it's bounced)
    let pending = queue.get_pending(10).await.unwrap();
    assert_eq!(pending.len(), 0);
}
