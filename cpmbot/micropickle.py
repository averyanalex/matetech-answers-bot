import pickle


def save_obj(obj, name):
    with open(f"{name}.pkl", "wb") as f:
        pickle.dump(obj, f)


def load_obj(name):
    with open(f"{name}.pkl", "rb") as f:
        return pickle.load(f)
