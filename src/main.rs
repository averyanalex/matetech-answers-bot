mod db;

use matetech_engine::MatetechError;
use once_cell::sync::Lazy;
use regex::Regex;
use sqlx::PgPool;
use teloxide::{
    adaptors::{throttle::Limits, Throttle},
    macros::BotCommands,
    prelude::*,
    utils::command::ParseError,
};
use tracing::instrument;

type Bot = Throttle<teloxide::Bot>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting database...");
    let db = sqlx::PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    sqlx::migrate!().run(&db).await?;

    tracing::info!("Starting bot...");
    let bot = teloxide::Bot::from_env()
        .throttle(Limits { messages_per_min_chat: 5, ..Default::default() });

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

#[derive(Debug, BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Доступные команды:")]
enum Command {
    #[command(
        description = "Войти в аккаунт. /login логин пароль",
        parse_with = "split"
    )]
    Login {
        login: String,
        password: String,
    },
    Speedrun {
        test_id: u32,
    },
    #[command(description = "Решить тест. /solve ссылка_на_тест", parse_with = parse_solve)]
    Solve {
        test_id: u32,
    },
    Broadcast {
        message: String,
    },
    Help,
}

fn parse_solve(input: String) -> Result<(u32,), ParseError> {
    static URL_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new("attempt_id=([0-9]+)").unwrap());

    if let Ok(num) = input.parse::<u32>() {
        return Ok((num,));
    }

    match URL_REGEX
        .captures(&input)
        .and_then(|c| c.get(1))
        .and_then(|c| c.as_str().parse::<u32>().ok())
    {
        Some(num) => Ok((num,)),
        None => Err(ParseError::Custom(
            format!("Couldn't parse /solve {input}").into(),
        )),
    }
}

#[instrument(skip(db, bot))]
async fn answer(
    db: PgPool,
    bot: Bot,
    msg: Message,
    cmd: Command,
) -> anyhow::Result<()> {
    match cmd {
        Command::Login { login, password } => {
            match matetech_engine::login(&login, &password).await {
                Ok(token) => {
                    db::set_token(&db, msg.chat.id.0, &token).await?;
                    bot.send_message(
                        msg.chat.id,
                        format!("Вы вошли в аккаунт {login}."),
                    )
                    .await?;
                }
                Err(err) => match err {
                    MatetechError::InvalidCredentials(_) => {
                        bot.send_message(
                            msg.chat.id,
                            "Неверный логин или пароль.",
                        )
                        .await?;
                    }
                    _ => {
                        return Err(err.into());
                    }
                },
            };
        }
        Command::Broadcast { message } => {
            if msg.chat.id == ChatId(1004106925) {
                for user in db::get_all_users(&db).await? {
                    if let Err(e) =
                        bot.send_message(ChatId(user), message.clone()).await
                    {
                        bot.send_message(msg.chat.id, e.to_string()).await?;
                    };
                }
            }
        }
        Command::Solve { test_id } | Command::Speedrun { test_id } => {
            let Some(token) = db::get_token(&db, msg.chat.id.0).await? else {
                bot.send_message(
                    msg.chat.id,
                    "Ознакомьтесь с инструкцией по использованию: \
                     /help.\nНеобходимо авторизовать бота в аккаунт \
                     дисткурсов.\n/login почта пароль.",
                )
                .await?;
                return Ok(());
            };

            let speedrun = matches!(cmd, Command::Speedrun { .. });

            let answers_msg = bot
                .send_message(
                    msg.chat.id,
                    if speedrun {
                        "ААА СПИДРАН ПО МАЙНКРАФТУ ПОЕХАЛИИИ"
                    } else {
                        "Решаем тест, это может занять до минуты..."
                    },
                )
                .await?;

            let solver = matetech_engine::Solver::new(token, test_id)?;
            match solver.solve(speedrun).await {
                Ok((answers_str, answers_set)) => {
                    for ans in answers_set {
                        db::save_answer(&db, &ans).await?;
                    }

                    bot.edit_message_text(
                        msg.chat.id,
                        answers_msg.id,
                        format!(
                            "Все ответы уже введены в тест, тем не менее \
                             рекомендуем их проверить:\n\n{answers_str}"
                        ),
                    )
                    .await?;
                }
                Err(err) => match err {
                    MatetechError::Forbidden(_) => {
                        bot.edit_message_text(
                            msg.chat.id,
                            answers_msg.id,
                            "Доступ к тесту невозможен. Убедитесь, что вы \
                             вошли в тот же аккаунт, с которого и запустили \
                             тест.",
                        )
                        .await?;
                    }
                    MatetechError::NotFound(_) => {
                        bot.edit_message_text(
                            msg.chat.id,
                            answers_msg.id,
                            "Тест не найден, проверьте корректность ссылки.",
                        )
                        .await?;
                    }
                    _ => {
                        bot.edit_message_text(
                            msg.chat.id,
                            answers_msg.id,
                            "Произошла неизвестная ошибка. Обратитесь о \
                             случившемся к @averyanalex",
                        )
                        .await?;
                        return Err(err.into());
                    }
                },
            }
        }
        Command::Help => {
            bot.send_message(msg.chat.id, HELP_TEXT).await?;
        }
    }

    Ok(())
}

const HELP_TEXT: &str =
    "\
Предупреждение: бот находится в стадии тестирования. Будьте готовы решить тест \
     самостоятельно в случае проблем.\n\nИнструкция по решению тестов.\n1. \
     Авторизуйте бота в аккаунт дисткурсов: /login почта пароль. Данные для \
     входа будут сохранены, в целях безопасности не рекомендуем использовать \
     этот же пароль на других сайтах.\n2. Начните любой тест и скопируйте \
     URL-адрес в адресной строке браузера.\n3. Отправьте ссылку на тест \
     боту.\n4. Подождите, пока бот выполнит тест.\n5. Бот автоматически \
     занесёт ответы в тест.\n6. Убедитесь в правильности ответов и завершите \
     тест.\n\nВ случае возникновения ошибок обращайтесь к @averyanalex";

async fn invalid_command(
    db: PgPool,
    bot: Bot,
    msg: Message,
) -> anyhow::Result<()> {
    let Some(text) = msg.text() else {
        answer(db, bot, msg, Command::Help).await?;
        return Ok(());
    };
    let Ok((test_id,)) = parse_solve(text.to_owned()) else {
        answer(db, bot, msg, Command::Help).await?;
        return Ok(());
    };
    answer(db, bot, msg, Command::Solve { test_id }).await?;
    Ok(())
}
