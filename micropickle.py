import pickle


def save_obj(obj, name):
    with open('./data/' + name + '.pkl', 'wb') as f:
        pickle.dump(obj, f)


def load_obj(name):
    with open('./data/' + name + '.pkl', 'rb') as f:
        return pickle.load(f)
