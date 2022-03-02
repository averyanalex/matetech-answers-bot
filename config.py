import os


def get_tg_token():
    return os.environ['TELEGRAM_TOKEN']


def get_cpm_token():
    return os.environ['CPM_TOKEN']
