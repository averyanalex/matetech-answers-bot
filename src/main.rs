mod db;
mod engine;

use sqlx::PgPool;
use teloxide::{prelude::*, utils::command::BotCommands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting database...");
    let db = sqlx::PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    sqlx::migrate!().run(&db).await?;

    tracing::info!("Starting bot...");
    let bot = Bot::from_env();

    let handler = Update::filter_message()
        .branch(dptree::entry().filter_command::<Command>().endpoint(answer))
        .branch(dptree::endpoint(invalid_command));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "ping the text back. /ping <text>")]
    Ping(String),
    #[command(
        description = "auth. /auth <login> <password>",
        parse_with = "split"
    )]
    Auth {
        login: String,
        password: String,
    },
    #[command(description = "solve. /solve <test_id>")]
    Solve {
        test_id: u32,
    },
    Help,
}

async fn answer(
    db: PgPool,
    bot: Bot,
    msg: Message,
    cmd: Command,
) -> anyhow::Result<()> {
    match cmd {
        Command::Ping(text) => {
            bot.send_message(
                msg.chat.id,
                if text.is_empty() {
                    "Pong!".to_owned()
                } else {
                    format!("Pong: {text}")
                },
            )
            .await?;
        }
        Command::Auth { login, password } => {
            let token = matetech_engine::login(login, password).await?;
            db::set_token(&db, msg.chat.id.0, &token).await?;
            bot.send_message(msg.chat.id, format!("Your token: {token}"))
                .await?;
            bot.send_message(msg.chat.id, format!("Your token: {token}"))
                .await?;
        }
        Command::Solve { test_id } => {
            let token = db::get_token(&db, msg.chat.id.0).await?;
            match token {
                Some(token) => {
                    let solver = matetech_engine::Solver::new(token, test_id)?;
                    let answers = solver.solve().await?;
                    bot.send_message(msg.chat.id, answers).await?;
                }
                None => {
                    bot.send_message(msg.chat.id, "Error, no token.").await?;
                }
            }
        }
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
    }

    Ok(())
}

async fn invalid_command(bot: Bot, msg: Message) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
}
