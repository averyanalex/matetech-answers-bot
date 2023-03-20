#!/usr/bin/env python
import requests
from . import micropickle
from .config import get_cpm_token


def get_answers(code: str):
    # Main result response
    response = requests.get(
        f"https://api.matetech.ru/api/public/companies/3/test_attempts/{code}/result",
        headers={"Authorization": get_cpm_token()},
    )
    if response.status_code == 404:
        return {"message": "Перепроверь айди", "channel": False}
    response = response.json()
    lesson_name = response["data"]["test_lesson"]["name"]

    # Course info
    response_course = requests.get(
        f"https://api.matetech.ru/api/public/companies/3/courses/{response['data']['test_lesson']['course_id']}?with=subcategories;coursesToOpen:name;availablePackagesBySort&withCount=lessons",
        headers={"Authorization": get_cpm_token()},
    )
    response_course = response_course.json()
    course_name = response_course["data"]["name"]

    result = course_name + "\n"
    result += lesson_name + "\n"

    # Class number
    cl = 0
    if course_name.find("8 кл") != -1:
        cl = 8
    if course_name.find("9 кл") != -1:
        cl = 9
    if course_name.find("10 кл") != -1:
        cl = 10
    if course_name.find("11 кл") != -1:
        cl = 11

    for q_id, question in enumerate(response["data"]["questions"][0]):
        pr = f"№{q_id + 1}: "

        if len(question["answers"]) == 1:
            # String answer
            pr += question["answers"][0]["value"]
        else:
            # Choose answer
            sorted_answers = sorted(question["answers"], key=lambda ans: ans["sort"])
            for a_id, answer in enumerate(sorted_answers):
                if answer["correct"]:
                    pr += str(a_id + 1)
        result += pr + "\n"

    # sent_tests = micropickle.load_obj("tests")
    sent_tests = []
    try:
        sent_tests.index(response["data"]["test_lesson"]["id"])
    except ValueError:
        sent_tests.append(response["data"]["test_lesson"]["id"])
        # micropickle.save_obj(sent_tests, "tests")
        return {"message": result, "channel": True, "cl": cl}
    return {"message": result, "channel": False}
