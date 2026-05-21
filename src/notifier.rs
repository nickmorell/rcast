#[allow(dead_code)]
pub fn notify_new_episodes(podcast_title: &str, count: usize) {
    let body = if count == 1 {
        format!("1 new episode in {podcast_title}")
    } else {
        format!("{count} new episodes in {podcast_title}")
    };
    let body = body.clone();
    std::thread::spawn(move || {
        let _ = notify_rust::Notification::new()
            .summary("RCast — New Episodes")
            .body(&body)
            .show();
    });
}

#[allow(dead_code)]
pub fn notify_download_complete(episode_title: &str) {
    let body = format!("Downloaded: {episode_title}");
    std::thread::spawn(move || {
        let _ = notify_rust::Notification::new()
            .summary("RCast — Download Complete")
            .body(&body)
            .show();
    });
}
