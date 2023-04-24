use tokio::time::{sleep, Duration};

pub async fn login(
    login: String,
    password: String,
) -> anyhow::Result<String> {
    sleep(Duration::from_secs(10)).await;
    Ok(format!("token for {login} {password}"))
}

pub async fn solve(
    _token: u32,
    test_id: String,
) -> anyhow::Result<String> {
    sleep(Duration::from_secs(10)).await;
    Ok(format!("answers for {test_id}"))
}
