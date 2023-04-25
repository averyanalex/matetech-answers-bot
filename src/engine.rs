use matetech_engine::MatetechError;
use tokio::time::{sleep, Duration};

pub async fn login(
    login: String,
    password: String,
) -> Result<String, MatetechError> {
    sleep(Duration::from_secs(2)).await;
    Ok(format!("token for {login} {password}"))
}

pub async fn solve(
    _token: String,
    test_id: u32,
) -> Result<String, MatetechError> {
    sleep(Duration::from_secs(10)).await;
    Ok(format!("answers for {test_id}"))
}
