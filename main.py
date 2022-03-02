import asyncio
import json
from telegram import Update
from telegram.ext import MessageHandler, Filters, Updater, CallbackContext
from engine import get_answers
from config import get_tg_token

updater = Updater(
    token=get_tg_token(), use_context=True)
dispatcher = updater.dispatcher


def messages_handler(update: Update, context: CallbackContext):
    try:
        msg = update.message.text
    except AttributeError:
        return
    try:
        answers = get_answers(msg)
        context.bot.send_message(
            chat_id=update.effective_chat.id, text=answers['message'])
        if answers['channel']:
            if answers['cl'] == 8:
                context.bot.send_message(
                    chat_id=-1001403887087, text=answers['message'])
            if answers['cl'] == 9:
                context.bot.send_message(
                    chat_id=-1001599892206, text=answers['message'])
            if answers['cl'] == 10:
                context.bot.send_message(
                    chat_id=-1001783255653, text=answers['message'])
            if answers['cl'] == 11:
                context.bot.send_message(
                    chat_id=-1001693528441, text=answers['message'])
    except json.decoder.JSONDecodeError:
        context.bot.send_message(chat_id=update.effective_chat.id, text='Ты мне какую что хрень кинул, перепроверь. '
                                                                        'Или админу напиши')


echo_handler = MessageHandler(
    Filters.text & (~Filters.command), messages_handler)
dispatcher.add_handler(echo_handler)

updater.start_polling()
