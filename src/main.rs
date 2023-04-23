mod db;

use matetech_engine;
use sqlx::PgPool;
use teloxide::{prelude::*, utils::command::BotCommands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting database...");
    let db = sqlx::PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    sqlx::migrate!().run(&db).await?;

    tracing::info!("Starting command bot...");
    let bot = Bot::from_env();

    // Command::repl(bot, answer).await;

    let handler = Update::filter_message()
        .branch(dptree::entry().filter_command::<Command>().endpoint(answer))
        .branch(dptree::endpoint(invalid_command));
    // .enter_dialogue::<Message, ErasedStorage<State>, State>()
    // .branch(dptree::case![State::Start].endpoint(start))
    // .branch(
    //     dptree::case![State::GotNumber(n)]
    //         .branch(dptree::entry().filter_command::<Command>().
    // endpoint(got_number))
    //         .branch(dptree::endpoint(invalid_command)),
    // );

    Dispatcher::builder(bot, handler)
        // Update::filter_message()
        //     .enter_dialogue::<Message, InMemStorage<State>,
        // State>()     .branch(dptree::case!
        // [State::Start].endpoint(start))
        //     .branch(dptree::case![State::ReceiveFullName].
        // endpoint(receive_full_name))
        //     .branch(dptree::case![State::ReceiveAge { full_name
        // }].endpoint(receive_age))     .branch(
        //         dptree::case![State::ReceiveLocation { full_name,
        // age }].endpoint(receive_location),     ),
        // .dependencies(dptree::deps![InMemStorage::<State>::new()])
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
    use Command::*;
    match cmd {
        Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Ping(text) => {
            bot.send_message(msg.chat.id, format!("Pong: {text}")).await?;
        }
        Auth { login, password } => {
            let token = matetech_engine::login(login, password).await?;
            db::set_token(&db, msg.chat.id.0, &token).await?;
            bot.send_message(msg.chat.id, format!("Your token: {token}"))
                .await?;
        }
        Solve { test_id } => {
            let token = db::get_token(&db, msg.chat.id.0).await?;
            match token {
                Some(token) => {
                    let answers =
                        matetech_engine::solve(test_id, token).await?;
                    bot.send_message(msg.chat.id, answers).await?;
                }
                None => {
                    bot.send_message(msg.chat.id, "Error, no token.").await?;
                }
            }
        }
    }

    Ok(())
}

async fn invalid_command(bot: Bot, msg: Message) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
}
