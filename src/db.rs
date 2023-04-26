use sqlx::PgPool;

pub async fn set_token(
    db: &PgPool,
    chat_id: i64,
    token: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
INSERT INTO tokens ( chat_id, token )
    VALUES ( $1, $2 )
    ON CONFLICT ( chat_id ) DO UPDATE
        SET token = $2
        "#,
        chat_id,
        token,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn save_answer(
    db: &PgPool,
    question_id: i64,
    answer: &String,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
INSERT INTO answers ( question_id, answer )
    VALUES ( $1, $2 )
    ON CONFLICT ( question_id ) DO UPDATE
        SET answer = $2
        "#,
        question_id,
        answer,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get_token(
    db: &PgPool,
    chat_id: i64,
) -> anyhow::Result<Option<String>> {
    let token = sqlx::query!(
        r#"
SELECT token
FROM tokens
WHERE chat_id = $1
        "#,
        chat_id
    )
    .fetch_optional(db)
    .await?;
    Ok(token.map(|r| r.token))
}
