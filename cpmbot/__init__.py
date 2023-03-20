import asyncio
import json
from telegram import Update
from telegram.ext import ApplicationBuilder, ContextTypes, MessageHandler
from telegram.ext.filters import TEXT
from .engine import get_answers
from .config import get_tg_token


async def messages_handler(update: Update, context: ContextTypes.DEFAULT_TYPE):
    try:
        msg = update.message.text
    except AttributeError:
        return
    try:
        answers = get_answers(msg)
        await context.bot.send_message(
            chat_id=update.effective_chat.id, text=answers["message"]
        )
        if answers["channel"]:
            if answers["cl"] == 8:
                context.bot.send_message(
                    chat_id=-1001403887087, text=answers["message"]
                )
            if answers["cl"] == 9:
                context.bot.send_message(
                    chat_id=-1001599892206, text=answers["message"]
                )
            if answers["cl"] == 10:
                context.bot.send_message(
                    chat_id=-1001783255653, text=answers["message"]
                )
            if answers["cl"] == 11:
                context.bot.send_message(
                    chat_id=-1001693528441, text=answers["message"]
                )
    except json.decoder.JSONDecodeError:
        await context.bot.send_message(
            chat_id=update.effective_chat.id,
            text="Ты мне какую что хрень кинул, перепроверь. " "Или админу напиши",
        )


def main():
    application = ApplicationBuilder().token(get_tg_token()).build()

    message_handler = MessageHandler(TEXT, messages_handler)
    application.add_handler(message_handler)

    application.run_polling()


if __name__ == "__main__":
    main()
