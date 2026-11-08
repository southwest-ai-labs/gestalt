use gestalt_acp::session::diff_notifier::DiffNotifier;

#[tokio::test]
async fn test_generate_diff_notification() {
    let path = "/test/file.rs";
    let before = "fn main() {\n    println!(\"Hello\");\n}\n";
    let after = "fn main() {\n    println!(\"Hello World\");\n}\n";

    let notification = DiffNotifier::generate_diff_notification(path, before, after);

    assert_eq!(notification.method, "session/fileUpdate");
    assert_eq!(notification.params.path, path);
    assert_eq!(notification.params.old_content, before);
    assert_eq!(notification.params.new_content, after);
    assert!(notification.params.diff.contains("--- a"));
    assert!(notification.params.diff.contains("+++ b"));
    assert!(notification.params.diff.contains("-    println!(\"Hello\");"));
    assert!(notification.params.diff.contains("+    println!(\"Hello World\");"));
}

#[tokio::test]
async fn test_empty_file_creation() {
    let path = "/test/new_file.txt";
    let before = "";
    let after = "Hello";

    let notification = DiffNotifier::generate_diff_notification(path, before, after);
    assert!(notification.params.diff.contains("+Hello"));
}

#[tokio::test]
async fn test_file_deletion() {
    let path = "/test/deleted_file.txt";
    let before = "Goodbye\n";
    let after = "";

    let notification = DiffNotifier::generate_diff_notification(path, before, after);
    assert!(notification.params.diff.contains("-Goodbye"));
}

#[tokio::test]
async fn test_broadcast_file_update() {
    let notifier = DiffNotifier::new(10);
    let mut rx1 = notifier.subscribe();
    let mut rx2 = notifier.subscribe();

    let path = "/test/broadcast.txt";
    let before = "A";
    let after = "B";

    notifier.broadcast_file_update(path, before, after).unwrap();

    let notif1 = rx1.recv().await.unwrap();
    let notif2 = rx2.recv().await.unwrap();

    assert_eq!(notif1.params.path, path);
    assert_eq!(notif2.params.path, path);
    assert_eq!(notif1.params.new_content, after);
    assert_eq!(notif2.params.new_content, after);
}
